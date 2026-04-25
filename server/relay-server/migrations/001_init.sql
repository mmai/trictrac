CREATE TABLE IF NOT EXISTS users (
    id            BIGSERIAL PRIMARY KEY,
    username      TEXT      NOT NULL UNIQUE,
    email         TEXT      NOT NULL UNIQUE,
    password_hash TEXT      NOT NULL,
    created_at    BIGINT    NOT NULL
);

CREATE TABLE IF NOT EXISTS game_records (
    id         BIGSERIAL PRIMARY KEY,
    game_id    TEXT      NOT NULL,
    room_code  TEXT      NOT NULL,
    started_at BIGINT    NOT NULL,
    ended_at   BIGINT,
    result     TEXT
);

CREATE TABLE IF NOT EXISTS game_participants (
    id             BIGSERIAL PRIMARY KEY,
    game_record_id BIGINT    NOT NULL REFERENCES game_records(id),
    user_id        BIGINT    REFERENCES users(id),
    player_id      BIGINT    NOT NULL,
    outcome        TEXT
);
