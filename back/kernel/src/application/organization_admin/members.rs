use serde_json::json;
use uuid::Uuid;

use crate::application::access::Authorizer;
use crate::application::auth::{NewUserToken, UserTokenPurpose};
use crate::application::mail::{MailLocale, Mailer, templates};
use crate::application::organization_admin::use_cases::{
    ALREADY_MEMBER, AdminChange, INVALID_MEMBER_STATUS, LAST_ADMIN, MAX_NAME_LENGTH,
    MEMBER_INVITATION_CANCELLED, MEMBER_INVITATION_RESENT, MEMBER_INVITED, MEMBER_NOT_FOUND,
    MEMBER_REACTIVATED, MEMBER_REMOVED, MEMBER_ROLE_CHANGED, MEMBER_SUSPENDED, NOT_INVITED, new_id,
    require_any,
};
use crate::application::organization_admin::{
    ChangeMemberRoleCommand, InviteMemberCommand, InvitedUser, ListWindow, Listing,
    MemberRepository, MemberView, NewInvitation, OrganizationAdminPorts, OrganizationAdminUseCases,
    RoleRepository, RoleSummary,
};
use crate::application::security::SecureTokens;
use crate::application::security::{NewSecurityEvent, SecurityEventKind, record_quietly};
use crate::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef, RecordType,
    TransactionManager, ensure_version,
};
use crate::domain::access::{AccessScope, Permission};
use crate::domain::users::UserStatus;
use crate::domain::{Email, NonEmptyString, RoleId, Secret, UserId};
use crate::errors::{AppError, FieldViolation};

impl<P> OrganizationAdminUseCases<P>
where
    P: OrganizationAdminPorts,
{
    pub async fn list_members(
        &self,
        scope: &AccessScope,
        window: ListWindow,
    ) -> Result<Listing<MemberView>, AppError> {
        require_any(
            scope,
            &[Permission::USERS_MANAGE, Permission::SETTINGS_MANAGE],
        )?;
        self.ports.members().list(window).await
    }

    pub async fn get_member(
        &self,
        scope: &AccessScope,
        user_id: &str,
    ) -> Result<MemberView, AppError> {
        scope.require(&Permission::USERS_MANAGE)?;
        self.require_member(user_id).await
    }

    pub async fn invite_member(
        &self,
        scope: &AccessScope,
        command: InviteMemberCommand,
    ) -> Result<MemberView, AppError> {
        scope.require(&Permission::USERS_MANAGE)?;

        let mut violations = Vec::new();
        let email = Email::new(command.email).ok();
        if email.is_none() {
            violations.push(FieldViolation::new(
                "email",
                "must be a valid email address",
            ));
        }
        let role = self.assignable_role(&command.role_id).await?;
        if role.is_none() {
            violations.push(FieldViolation::new("role_id", "role was not found"));
        }
        let (Some(email), Some(role)) = (email, role) else {
            return Err(AppError::invalid_fields("invalid invitation", violations));
        };

        if self.ports.members().email_exists(&email).await? {
            return Err(AppError::conflict_code(
                ALREADY_MEMBER,
                "this user already belongs to the organization",
            ));
        }
        let user = InvitedUser {
            id: UserId::new(new_id())?,
            display_name: display_name_from_email(&email)?,
            email: email.clone(),
        };
        let user_id = user.id.clone();
        let (token, secret) = self.invitation_token(&user_id)?;
        let role_payload = json!({ "role_id": role.id.as_str(), "email": email.as_str() });
        let transactions = self.ports.transactions();
        let mut tx = transactions.begin().await?;
        self.ports
            .members()
            .create_invitation(
                &mut tx,
                NewInvitation {
                    user,
                    role_id: role.id,
                    invited_by: scope.user_id().clone(),
                    token,
                },
            )
            .await?;
        self.record_admin_change(
            &mut tx,
            scope,
            AdminChange::new(
                MEMBER_INVITED,
                RecordType::MEMBER,
                user_id.as_str(),
                role_payload,
            )?,
        )
        .await?;
        transactions.commit(tx).await?;
        self.send_invitation(scope, email.as_str(), &secret).await?;
        self.require_member(user_id.as_str()).await
    }

    pub async fn resend_invitation(
        &self,
        scope: &AccessScope,
        user_id: &str,
    ) -> Result<MemberView, AppError> {
        scope.require(&Permission::USERS_MANAGE)?;
        let member = self.require_member(user_id).await?;
        if member.status != UserStatus::Invited {
            return Err(not_invited());
        }

        let (token, secret) = self.invitation_token(&member.user_id)?;
        let transactions = self.ports.transactions();
        let mut tx = transactions.begin().await?;
        self.ports
            .members()
            .replace_invitation_token(&mut tx, &member.user_id, token, self.now())
            .await?;
        self.record_admin_change(
            &mut tx,
            scope,
            AdminChange::new(
                MEMBER_INVITATION_RESENT,
                RecordType::MEMBER,
                member.user_id.as_str(),
                json!({ "email": member.email }),
            )?,
        )
        .await?;
        transactions.commit(tx).await?;
        self.send_invitation(scope, &member.email, &secret).await?;
        Ok(member)
    }

    pub async fn cancel_invitation(
        &self,
        scope: &AccessScope,
        user_id: &str,
    ) -> Result<(), AppError> {
        scope.require(&Permission::USERS_MANAGE)?;
        let member = self.require_member(user_id).await?;
        if member.status != UserStatus::Invited {
            return Err(not_invited());
        }
        let transactions = self.ports.transactions();
        let mut tx = transactions.begin().await?;
        if !self
            .ports
            .members()
            .cancel_invitation(&mut tx, &member.user_id, self.now())
            .await?
        {
            return Err(not_invited());
        }
        self.record_admin_change(
            &mut tx,
            scope,
            AdminChange::new(
                MEMBER_INVITATION_CANCELLED,
                RecordType::MEMBER,
                member.user_id.as_str(),
                json!({ "email": member.email }),
            )?,
        )
        .await?;
        transactions.commit(tx).await?;
        Ok(())
    }

    pub async fn change_member_role(
        &self,
        scope: &AccessScope,
        user_id: &str,
        command: ChangeMemberRoleCommand,
    ) -> Result<MemberView, AppError> {
        scope.require(&Permission::USERS_MANAGE)?;
        let member = self.require_member(user_id).await?;
        ensure_version(command.expected_updated_at, member.updated_at)?;
        let role = self
            .assignable_role(&command.role_id)
            .await?
            .ok_or_else(|| {
                AppError::invalid_fields(
                    "invalid role",
                    vec![FieldViolation::new("role_id", "role was not found")],
                )
            })?;
        if role.id == member.role.id {
            return Ok(member);
        }
        if !role.is_system_admin() {
            self.ensure_not_last_admin(&member).await?;
        }

        let record = RecordRef::new(RecordType::MEMBER, record_id(&member.user_id)?);
        let actor = Actor::from_scope(scope);
        let now = self.now();
        let event = DomainEvent::new(actor.clone(), MEMBER_ROLE_CHANGED, now)
            .with_record(record.clone())
            .with_payload(json!({
                "user_id": member.user_id.as_str(),
                "old_role_id": member.role.id.as_str(),
                "new_role_id": role.id.as_str(),
            }));
        let entry = AuditEntry::new(actor, MEMBER_ROLE_CHANGED, now)
            .with_record(record)
            .with_changes(vec![FieldChange::new(
                "role",
                json!(member.role.name),
                json!(role.name),
            )])
            .with_ip(command.client.ip.clone());
        let security_event = NewSecurityEvent::new(SecurityEventKind::PermissionChanged, now)
            .user(&member.user_id)
            .client(
                command.client.ip.as_deref(),
                command.client.user_agent.as_deref(),
            )
            .metadata(json!({
                "change": "member_role",
                "old_role": member.role.name,
                "new_role": role.name,
                "actor_user_id": scope.user_id().as_str(),
            }));

        let transactions = self.ports.transactions();
        let mut tx = transactions.begin().await?;
        self.ports
            .members()
            .change_role(&mut tx, &member.user_id, &role.id)
            .await?;
        self.ports.events().publish(&mut tx, event).await?;
        self.ports.audit().record(&mut tx, entry).await?;
        transactions.commit(tx).await?;
        record_quietly(&self.security_events, security_event).await;

        self.require_member(user_id).await
    }

    pub async fn suspend_member(
        &self,
        scope: &AccessScope,
        user_id: &str,
    ) -> Result<MemberView, AppError> {
        scope.require(&Permission::USERS_MANAGE)?;
        let member = self.require_member(user_id).await?;
        if member.status != UserStatus::Active {
            return Err(invalid_status("only active members can be suspended"));
        }
        self.ensure_not_last_admin(&member).await?;
        let transactions = self.ports.transactions();
        let mut tx = transactions.begin().await?;
        self.ports
            .members()
            .deactivate(&mut tx, &member.user_id, self.now())
            .await?;
        self.record_admin_change(
            &mut tx,
            scope,
            AdminChange::new(
                MEMBER_SUSPENDED,
                RecordType::MEMBER,
                member.user_id.as_str(),
                json!({ "email": member.email }),
            )?,
        )
        .await?;
        transactions.commit(tx).await?;
        self.require_member(user_id).await
    }

    pub async fn reactivate_member(
        &self,
        scope: &AccessScope,
        user_id: &str,
    ) -> Result<MemberView, AppError> {
        scope.require(&Permission::USERS_MANAGE)?;
        let member = self.require_member(user_id).await?;
        if member.status != UserStatus::Deactivated {
            return Err(invalid_status(
                "only deactivated members can be reactivated",
            ));
        }
        let transactions = self.ports.transactions();
        let mut tx = transactions.begin().await?;
        self.ports
            .members()
            .reactivate(&mut tx, &member.user_id)
            .await?;
        self.record_admin_change(
            &mut tx,
            scope,
            AdminChange::new(
                MEMBER_REACTIVATED,
                RecordType::MEMBER,
                member.user_id.as_str(),
                json!({ "email": member.email }),
            )?,
        )
        .await?;
        transactions.commit(tx).await?;
        self.require_member(user_id).await
    }

    pub async fn remove_member(&self, scope: &AccessScope, user_id: &str) -> Result<(), AppError> {
        scope.require(&Permission::USERS_MANAGE)?;
        let member = self.require_member(user_id).await?;
        self.ensure_not_last_admin(&member).await?;
        let transactions = self.ports.transactions();
        let mut tx = transactions.begin().await?;
        self.ports
            .members()
            .remove(&mut tx, &member.user_id, self.now())
            .await?;
        self.record_admin_change(
            &mut tx,
            scope,
            AdminChange::new(
                MEMBER_REMOVED,
                RecordType::MEMBER,
                member.user_id.as_str(),
                json!({ "email": member.email }),
            )?,
        )
        .await?;
        transactions.commit(tx).await?;
        Ok(())
    }

    async fn require_member(&self, user_id: &str) -> Result<MemberView, AppError> {
        let Ok(user_id) = UserId::new(user_id) else {
            return Err(member_not_found());
        };
        self.ports
            .members()
            .find(&user_id)
            .await?
            .ok_or_else(member_not_found)
    }

    async fn assignable_role(&self, role_id: &str) -> Result<Option<RoleSummary>, AppError> {
        let Ok(role_id) = RoleId::new(role_id) else {
            return Ok(None);
        };
        Ok(self
            .ports
            .roles()
            .find(&role_id)
            .await?
            .map(|view| view.role))
    }

    async fn ensure_not_last_admin(&self, member: &MemberView) -> Result<(), AppError> {
        if member.is_active_admin() && self.ports.members().count_active_admins().await? <= 1 {
            return Err(AppError::conflict_code(
                LAST_ADMIN,
                "the organization needs at least one active administrator",
            ));
        }
        Ok(())
    }

    fn invitation_token(&self, user_id: &UserId) -> Result<(NewUserToken, Secret), AppError> {
        let issued = self.ports.secrets().issue()?;
        let token = NewUserToken {
            id: new_id(),
            user_id: user_id.clone(),
            purpose: UserTokenPurpose::Invitation,
            token_hash: issued.hash,
            payload: None,
            expires_at: self.now() + self.settings.invitation_ttl,
        };
        Ok((token, issued.token))
    }

    async fn send_invitation(
        &self,
        scope: &AccessScope,
        email: &str,
        token: &Secret,
    ) -> Result<(), AppError> {
        let context = self
            .ports
            .members()
            .invitation_context(scope.user_id(), email)
            .await?;
        let message = templates::invitation(
            email,
            &context.inviter_display_name,
            &context.organization_name,
            &self.settings.link("/accept-invitation", token),
            MailLocale::resolve(
                context.invitee_locale.as_deref(),
                Some(&context.organization_locale),
            ),
        );
        if let Err(error) = self.ports.mailer().send(message).await {
            tracing::warn!(%error, "failed to send invitation email");
        }
        Ok(())
    }
}

fn display_name_from_email(email: &Email) -> Result<NonEmptyString, AppError> {
    let local_part = email.as_str().split('@').next().unwrap_or_default();
    let name: String = local_part.chars().take(MAX_NAME_LENGTH).collect();
    Ok(NonEmptyString::new(name, "display_name")?)
}

fn record_id(user_id: &UserId) -> Result<Uuid, AppError> {
    Uuid::parse_str(user_id.as_str())
        .map_err(|_| AppError::Internal("user id is not a UUID".to_owned()))
}

fn member_not_found() -> AppError {
    AppError::not_found_code(MEMBER_NOT_FOUND, "member was not found")
}

fn not_invited() -> AppError {
    AppError::conflict_code(NOT_INVITED, "this member has no pending invitation")
}

fn invalid_status(message: &str) -> AppError {
    AppError::conflict_code(INVALID_MEMBER_STATUS, message.to_owned())
}
