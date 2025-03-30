CREATE TABLE conversations(
    id TEXT NOT NULL PRIMARY KEY,
    context_key TEXT NULL UNIQUE,
    content BLOB NOT NULL
);

CREATE TABLE skeb_illusts(
    url TEXT NOT NULL PRIMARY KEY,
    creator_name TEXT NOT NULL,
    comment TEXT NOT NULL
);
