# Repeated Entries

```postgresql
with timeline_items as (select jsonb_array_elements(json -> 'timelineItems') - 'samples' as entry
                        from raw_files)
select entry ->> 'itemId'    as item_id,
       entry ->> 'lastSaved' as last_saved,
       entry ->> 'endDate'   as end_date,
       entry ->> 'startDate' as start_date
from timeline_items
WHERE entry ->> 'itemId' IN (SELECT entry ->> 'itemId'
                             FROM timeline_items
                             GROUP BY entry ->> 'itemId'
                             HAVING COUNT(entry ->> 'itemId') > 1);
```

```postgresql
select md5(rfe.entry)
from (select jsonb_array_elements(json -> 'timelineItems') - 'samples' as entry
      from raw_files) as rfe
where rfe.entry ->> 'itemId' = '01300b32-a3f7-4911-ab63-a0cbf0f312e0';
```

```csv
c7e2a142aabdcdddd00143da6612c235
c7e2a142aabdcdddd00143da6612c235
```

running this query we see that the JSON content of the repeated entry is the same so it can be discounted.
