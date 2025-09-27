CREATE TABLE IF NOT EXISTS event (
    id INT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    emoji VARCHAR(58) -- 5 chars for `<a::>` + name is 32 chars + 20 for snowflake ID
);
