create materialized view directory_paths (directory_id, path) as
    with recursive paths (directory_id, path) as (
        select id, concat('/', slug)
            from directories
            where parent_directory_id is null
        union all
        select directories.id, concat(parent.path, '/', directories.slug)
            from directories
            join paths parent
                on directories.parent_directory_id = parent.directory_id
    )
    select * from paths;

create materialized view post_paths (post_id, path) as
    select posts.id, concat(parent.path, '/', posts.slug)
        from posts
        join directory_paths parent
            on posts.directory_id = parent.directory_id;

create unique index on directory_paths (directory_id);
create unique index on directory_paths (path);
create unique index on post_paths (post_id);
create unique index on post_paths (path);
