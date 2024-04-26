with timelineItems as (select jsonb_array_elements(raw_files.json -> 'timelineItems') as timelineItem
                       from raw_files
                       where date = ANY ($1)),
     max_last_saved as (select timelineItem ->> 'itemId'                          as itemId,
                               max((timelineItem ->> 'lastSaved') :: timestamptz) as latest_last_saved
                        from timelineItems
                        group by timelineItem ->> 'itemId'),
     unique_timline_items as (select distinct on (max_last_saved.itemId) *
                              from max_last_saved
                                       inner join timelineItems
                                                  on timelineItems.timelineItem ->> 'itemId' = max_last_saved.itemId
                                                      and (timelineItems.timelineItem ->> 'lastSaved') :: timestamptz =
                                                          max_last_saved.latest_last_saved)
insert
into public.timeline_item (item_id, json, place_id, end_date, last_saved, server_last_updated)
select unique_timline_items.itemId :: uuid                                  as item_id,
       unique_timline_items.timelineItem                                    as json,
       (unique_timline_items.timelineItem -> 'place' ->> 'placeId') :: uuid as place_id,
       (unique_timline_items.timelineItem ->> 'endDate') :: timestamptz     as end_date,
       unique_timline_items.latest_last_saved :: timestamptz                as last_saved,
       now()                                                                as server_last_updated
from unique_timline_items
on conflict (item_id) do update set json                = excluded.json,
                                    place_id            = excluded.place_id,
                                    end_date            = excluded.end_date,
                                    last_saved          = excluded.last_saved,
                                    server_last_updated = excluded.server_last_updated
where excluded.last_saved > public.timeline_item.last_saved
returning item_id;
