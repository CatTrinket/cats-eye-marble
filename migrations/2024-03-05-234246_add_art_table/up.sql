create table directories (
    id serial primary key,
    title text not null,
    slug text not null,
    parent_directory_id int,

    foreign key (parent_directory_id) references directories (id),
    unique (parent_directory_id, slug)
);

create table posts (
    id serial primary key,
    title text not null,
    slug text not null,
    timestamp timestamp not null,
    directory_id int not null,
    description text not null,

    foreign key (directory_id) references directories (id),
    foreign key (author_id) references authors (id),
    unique (directory_id, slug)
);

create table post_images (
    id serial primary key,
    post_id integer not null,
    "order" integer not null,
    alt_text text not null,

    foreign key (post_id) references posts (id),
    unique (post_id, "order")
);
