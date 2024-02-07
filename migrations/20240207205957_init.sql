CREATE TABLE IF NOT EXISTS place
(
    place_id UUID PRIMARY KEY not null,
    json     JSONB            NOT NULL
);

CREATE TABLE IF NOT EXISTS timeline_item
(
    item_id  UUID PRIMARY KEY         not null,
    json     JSONB                    NOT NULL,
    place_id UUID,
    end_date TIMESTAMP WITH TIME ZONE NOT NULL,
    foreign key (place_id) references place (place_id)
);

CREATE TABLE IF NOT EXISTS raw_files
(
    date   TEXT PRIMARY KEY not null,
    sha256 TEXT             not null,
    json   JSONB            NOT NULL
)
