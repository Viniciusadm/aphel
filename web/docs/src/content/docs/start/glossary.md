---
title: Glossary
description: Alphabetical definitions of the terms used across the Aphel documentation, each linked to the page that explains it.
sidebar:
  order: 5
---

## A

**Access scope**
: The identity a piece of work runs as: the user, their role, their teams and their effective permissions. Every use case receives one and checks it. Background work without a user runs as the system. See [Architecture](/_aphel/docs/start/architecture/).

**Action (automation)**
: A step an automation performs after its trigger fires and its conditions pass, such as creating or updating a record. Modules and plugins contribute actions. See [Automations](/_aphel/docs/using/automations/) and [Automation and widgets](/_aphel/docs/plugins/automation-and-widgets/).

**Action hook**
: A hook that reacts to something that happened and cannot change the data or block the operation. See [Hooks](/_aphel/docs/plugins/hooks/).

**Actor**
: Who caused a change: a user, the system, an API token, an automation, a plugin or a named service. Recorded on events and audit entries. See [Events](/_aphel/docs/plugins/events/).

**API token**
: A credential for non-browser clients, sent as `Authorization: Bearer`. Its permissions are its scopes intersected with the creator's current permissions. See [Authentication](/_aphel/docs/reference/authentication/).

**Audit log**
: The record of who changed what and when, with field-level differences and sensitive values hidden. Written in the same transaction as the change. See [Security](/_aphel/docs/admin/security/).

## B

**Backend bundle (`backend.wasm`)**
: The WebAssembly component inside a runtime plugin that runs hooks, event handlers, routes and schedules. See [Lifecycle](/_aphel/docs/plugins/lifecycle/).

**Blueprint**
: The identity of a system: which modules are required and recommended, the navigation, extra roles and the configuration packages to apply. Blueprints can extend one another. See [Blueprints](/_aphel/docs/admin/blueprints/).

## C

**Capability**
: What a runtime plugin is allowed to do, declared in its manifest through the permissions it requires and defines. Every plugin call runs with the actor's permissions intersected with the plugin's capabilities. See [Permissions and roles](/_aphel/docs/plugins/permissions-and-roles/).

**Committed hook**
: The record hook that runs after the write transaction has committed. A failure is logged and does not fail the request. See [Architecture](/_aphel/docs/start/architecture/).

**Configuration package**
: A portable bundle of organization configuration, such as fields, pipelines and roles, applied with a preview of what will be created or updated. Installing one never deletes anything and never touches business records. See [Configuration packages](/_aphel/docs/admin/config-packages/).

**Consumer (event consumer)**
: A component that handles domain events from the outbox. It has a stable name and declares which event types it accepts. Delivery is at-least-once. See [Events](/_aphel/docs/plugins/events/).

**Contract**
: A description of what a capability offers, in a small crate with no behavior. Modules and plugins depend on contracts, never on each other. See [Architecture](/_aphel/docs/start/architecture/).

**CSRF header**
: The `X-Requested-With: aphel` header required on requests that change data when they are not authenticated with a bearer token. See [Authentication](/_aphel/docs/reference/authentication/).

**Custom field**
: A field added to a record type by an administrator, stored in typed, indexed columns. See [Custom fields](/_aphel/docs/using/custom-fields/).

**Custom object**
: A record type created by an administrator, with the key `custom.<key>`. See [Concepts](/_aphel/docs/start/concepts/).

## D

**Domain event**
: A message published for every business change, in the same transaction as the change. Events feed webhooks, automations, notifications, search and plugins. See [Events](/_aphel/docs/plugins/events/) and [Webhooks and events](/_aphel/docs/reference/webhooks-and-events/).

## E

**Entity**
: Another name for a record type, used where the data model is discussed. See [Entities and fields](/_aphel/docs/plugins/entities-and-fields/).

**Entity registry**
: The in-memory description of every record type and field, used by lists, filters, search, import, export, automations and the API. Rebuilt when objects or fields change. See [Architecture](/_aphel/docs/start/architecture/).

## F

**Field permission**
: A per-role setting that makes a field hidden or read-only. See [Roles and permissions](/_aphel/docs/admin/roles-and-permissions/).

**Filter hook**
: A hook that receives data and returns a changed version, such as the values of a record before it is saved. See [Hooks](/_aphel/docs/plugins/hooks/).

## H

**Hook**
: A synchronous extension point inside the request. Hooks are actions, vetoes or filters. See [Hooks](/_aphel/docs/plugins/hooks/) and [Hooks catalog](/_aphel/docs/reference/hooks-catalog/).

## I

**Idempotency key**
: A value sent in the `Idempotency-Key` header of a `POST` so that a retry returns the original result instead of repeating the action. See [REST API](/_aphel/docs/reference/rest-api/).

**Installation**
: One running Aphel with its own database. It serves exactly one company. See [Concepts](/_aphel/docs/start/concepts/).

**Installer**
: The setup wizard, available only until the first company is created. It creates the organization and the first administrator and applies the chosen blueprint. See [Quickstart](/_aphel/docs/start/quickstart/).

## J

**Job**
: A unit of background work stored in the MySQL queue and run by the worker, with retries. See [Operations](/_aphel/docs/admin/operations/).

## K

**Kernel**
: The always-on core of the platform: identity, access, records, events, jobs, files, audit, settings and the module and plugin hosts. It knows no business domain. See [Architecture](/_aphel/docs/start/architecture/).

## M

**Managed entity**
: A record type declared by a runtime plugin whose tables, columns and indexes the host generates from the declaration. Plugins never write SQL. See [Entities and fields](/_aphel/docs/plugins/entities-and-fields/).

**Manifest (`plugin.toml`)**
: The file at the root of a runtime plugin that declares everything it contributes and requires. See [Manifest](/_aphel/docs/plugins/manifest/).

**Module**
: A feature compiled into the binary, such as people, deals or email, that administrators enable and disable. See [Modules](/_aphel/docs/admin/modules/) and [Module catalog](/_aphel/docs/reference/module-catalog/).

## O

**Organization**
: The one company an installation serves. Its settings are stored once for the whole installation. Not to be confused with the organization record type, which represents a customer company. See [Concepts](/_aphel/docs/start/concepts/).

**Outbox**
: The table where domain events are written in the same transaction as the change, so that none is lost. A dispatcher turns them into consumer jobs. See [Architecture](/_aphel/docs/start/architecture/).

## P

**Permission**
: A named capability such as `deals.read`. Record permissions carry a scope; administrative permissions do not. See [Permissions catalog](/_aphel/docs/reference/permissions-catalog/).

**Plugin (runtime plugin)**
: A `.aphelplugin` package with a manifest, a WebAssembly backend and an optional web bundle, installed without rebuilding or restarting. See [Plugins overview](/_aphel/docs/plugins/overview/).

**Public app**
: A web application a plugin serves under `/p/{key}/` for people who are not signed in. See [Public web apps](/_aphel/docs/plugins/public-web-apps/).

**Public event**
: A domain event that is part of the documented catalog delivered to outgoing webhooks. See [Webhooks and events](/_aphel/docs/reference/webhooks-and-events/).

## R

**Record**
: One row of business data, such as a person or a deal, belonging to a record type. See [Records and views](/_aphel/docs/using/records-and-views/).

**Record pipeline**
: The sequence every record write goes through: authorization, saving, write, validation, saved, event and audit, commit, committed. See [Architecture](/_aphel/docs/start/architecture/).

**Record type**
: The kind of a record, identified by a key such as `people.person` or `custom.vehicle`. Registered in the entity registry. See [Concepts](/_aphel/docs/start/concepts/).

**Relationship**
: A configurable link between two record types with a cardinality and labels on both ends. See [Custom fields](/_aphel/docs/using/custom-fields/).

**Role**
: A set of permissions, each with a scope, assigned to users. The system roles are `admin`, `manager` and `member`. See [Roles and permissions](/_aphel/docs/admin/roles-and-permissions/).

## S

**Saved view**
: A named combination of filters, sorting and columns, personal, shared with a team or shared with the organization. See [Records and views](/_aphel/docs/using/records-and-views/).

**Schedule**
: A recurring task defined by a cron expression with a seconds field, run by the scheduler. See [Schedules](/_aphel/docs/plugins/schedules/).

**Scheduler**
: The part of the worker that creates recurring jobs from schedules. See [Operations](/_aphel/docs/admin/operations/).

**Scope (record scope)**
: The reach of a record permission: `own`, `team` or `organization`. See [Concepts](/_aphel/docs/start/concepts/).

**Secret vault**
: The encrypted store for passwords, keys and tokens of integrations and plugins, protected by `APP_ENCRYPTION_KEY`. Secrets are never returned by the API. See [Settings and secrets](/_aphel/docs/plugins/settings-and-secrets/).

**Session**
: A signed-in browser, represented by an opaque token in the `aphel_session` cookie. See [Security](/_aphel/docs/admin/security/).

**Slot**
: A named place in a web screen where other web modules or plugins can add content. See [Web screens](/_aphel/docs/plugins/web-screens/).

## T

**Team**
: A group of users, which can contain sub-teams. Used by the `team` scope. See [Users and teams](/_aphel/docs/admin/users-and-teams/).

**Trigger**
: The event or schedule that starts an automation. See [Automations](/_aphel/docs/using/automations/).

**Two-factor authentication**
: A second sign-in step using a time-based one-time code, with recovery codes. See [Security](/_aphel/docs/admin/security/).

## U

**UI plugin**
: A runtime plugin that declares `[ui]` and provides an alternative interface that every user can choose. See [UI plugins](/_aphel/docs/plugins/ui-plugins/).

## V

**Veto hook**
: A hook that can block an operation by returning an error. See [Hooks](/_aphel/docs/plugins/hooks/).

## W

**Web bundle**
: The ES module build in the `web/` folder of a runtime plugin that provides its screens. See [Web bundle](/_aphel/docs/plugins/web-bundle/).

**Webhook**
: An outgoing webhook delivers events to an external URL, signed and retried. An incoming webhook is a secret URL that lets an external system trigger actions. See [Integrations](/_aphel/docs/admin/integrations/).

**Worker**
: The process (`aphel worker`, or `aphel serve --with-worker`) that runs jobs and the scheduler. See [Operations](/_aphel/docs/admin/operations/).
