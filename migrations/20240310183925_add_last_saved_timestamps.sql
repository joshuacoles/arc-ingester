-- Add last_saved columns

-- timeline_item
alter table timeline_item
    add column last_saved timestamptz;

update timeline_item
set last_saved = (json ->> 'lastSaved') :: timestamptz
where last_saved is null;

alter table timeline_item
    alter column last_saved set not null;

-- place

alter table place
    add column last_saved timestamptz;

update place
set last_saved = (json ->> 'lastSaved') :: timestamptz
where last_saved is null;

alter table place
    alter column last_saved set not null;

-- Add server_last_updated columns

alter table timeline_item
    add column server_last_updated timestamptz not null default now();

alter table place
    add column server_last_updated timestamptz not null default now();
