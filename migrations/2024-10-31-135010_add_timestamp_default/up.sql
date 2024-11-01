-- I'm realizing that I was mistaken about the timezone types.  I thought
-- timestamp with time zone would store an offset from UTC along with the
-- timestamp.  In fact, it *accepts* times with an offset but then just
-- converts it to UTC and discards the input offset.  Huh.

-- So I should actually be using timestamp with time zone.  I'll fix it some
-- other time.  This does the right thing for now.

alter table posts alter column timestamp set default now() at time zone 'UTC';
