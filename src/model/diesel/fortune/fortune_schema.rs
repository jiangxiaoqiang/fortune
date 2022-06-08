table! {
    bill_book_template (id) {
        id -> Int4,
        created_time -> Int8,
        updated_time -> Int8,
        deleted -> Int4,
        remark -> Varchar,
        name -> Varchar,
        tags -> Varchar,
        slogan -> Varchar,
        icon_url -> Varchar,
        user_count -> Int8,
        template_type -> Int4,
    }
}

table! {
    bill_record (id) {
        id -> Int8,
        created_time -> Int8,
        updated_time -> Int8,
        deleted -> Int4,
        user_id -> Int8,
        bill_book_id -> Int8,
        remark -> Nullable<Varchar>,
        amount -> Int8,
    }
}

table! {
    fortune_contents (id) {
        id -> Int4,
        parent_id -> Int4,
        created_time -> Int8,
        updated_time -> Int8,
        name -> Varchar,
        contents_type -> Int4,
        deleted -> Int4,
        hidden -> Int4,
        sort -> Int4,
        contents_source -> Nullable<Int4>,
    }
}

allow_tables_to_appear_in_same_query!(
    bill_book_template,
    bill_record,
    fortune_contents,
);
