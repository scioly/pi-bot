CREATE TABLE IF NOT EXISTS scioly_user_stats (
    id INT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    discord_user_id BIGINT UNSIGNED NOT NULL,
    phpbb_user_id INT UNSIGNED NOT NULL,
    username VARCHAR(255) NOT NULL,
    forums_avatar TEXT,
    forums_post_count INT UNSIGNED NOT NULL,
    forums_thanks_received INT UNSIGNED NOT NULL,
    forums_thanks_given INT UNSIGNED NOT NULL,
    wiki_edit_count INT UNSIGNED NOT NULL,
    test_ex_upload_count INT UNSIGNED NOT NULL,
    gallery_score INT UNSIGNED NOT NULL,
    gallery_post_count INT UNSIGNED NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW() ON UPDATE NOW(),
    UNIQUE INDEX (discord_user_id),
    UNIQUE INDEX (phpbb_user_id),
    INDEX (username)
);
