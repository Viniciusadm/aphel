UPDATE users SET email_verified_at = COALESCE(created_at, CURRENT_TIMESTAMP(6)) WHERE email_verified_at IS NULL;
