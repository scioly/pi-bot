CREATE TABLE IF NOT EXISTS authorization_request (
    id INT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    discord_user_id BIGINT UNSIGNED NOT NULL,
    state_code BINARY(255) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP NOT NULL DEFAULT (DATE_ADD(NOW(), INTERVAL 10 MINUTE)),
    UNIQUE INDEX (discord_user_id),
    UNIQUE INDEX (state_code)
);

CREATE TABLE IF NOT EXISTS scioly_tokens (
    id INT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    discord_user_id BIGINT UNSIGNED NOT NULL,
    phpbb_user_id INT UNSIGNED NOT NULL,
    access_token BINARY(255) NOT NULL,
    refresh_token BINARY(255) NOT NULL,
    received_at TIMESTAMP NOT NULL DEFAULT NOW(),
    access_expires_at TIMESTAMP NOT NULL,
    -- refresh_expires_at TIMESTAMP NOT NULL, -- current version of the API does not give this
    UNIQUE INDEX (discord_user_id),
    UNIQUE INDEX (phpbb_user_id),
    UNIQUE INDEX (access_token),
    UNIQUE INDEX (refresh_token)
);
