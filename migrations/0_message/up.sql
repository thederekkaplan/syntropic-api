CREATE TABLE "message"
(
    "id"        bytea PRIMARY KEY,
    "timestamp" timestamptz NOT NULL,
    "body"      text        NOT NULL
);
