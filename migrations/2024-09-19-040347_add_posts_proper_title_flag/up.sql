alter table posts add column has_proper_title boolean not null default false;
update posts set has_proper_title = true;

alter table directories
    add column has_proper_title boolean not null default false;
