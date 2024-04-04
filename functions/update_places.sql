with timelineItems as (select jsonb_array_elements(raw_files.json -> 'timelineItems') as timelineItem
                       from raw_files),
     places as (select distinct on (md5(timelineItem ->> 'place' :: text)) timelineItem -> 'place'                                  as place,
                                                                           timelineItem -> 'place' ->> 'placeId'                    as placeId,
                                                                           (timelineItem -> 'place' ->> 'lastSaved') :: timestamptz as lastSaved
                from timelineItems
                where timelineItem ->> 'place' is not null),
     places_with_max_last_saved as (select place -> 'placeId'                          as placeId,
                                           max((place ->> 'lastSaved') :: timestamptz) as latest_last_saved
                                    from places
                                    group by place -> 'placeId'),
     latest_places as (select places.*
                       from places_with_max_last_saved
                                inner join places on places.place -> 'placeId' = places_with_max_last_saved.placeId and
                                                     places.lastSaved =
                                                     places_with_max_last_saved.latest_last_saved)
insert
into public.place (place_id, json, last_saved, server_last_updated)
select (placeId :: uuid) as place_id, place as json, lastSaved as last_saved, now() as server_last_updated
from latest_places
on conflict (place_id) do update set json                = excluded.json,
                                     last_saved          = excluded.last_saved,
                                     server_last_updated = excluded.server_last_updated
where excluded.last_saved > public.place.last_saved;
