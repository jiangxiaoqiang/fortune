use diesel::{ ExpressionMethods, QueryDsl, RunQueryDsl, TextExpressionMethods};
use rocket::serde::json::Json;
use rust_wheel::common::util::time_util::get_current_millisecond;
use rust_wheel::config::db::config;
use rust_wheel::model::user::login_user_info::LoginUserInfo;
use crate::model::diesel::fortune::fortune_custom_models::{BillBookAccountAdd, BillBookAdd, BillBookContentAdd, BillBookRoleAdd};

use crate::model::diesel::fortune::fortune_models::{Account, BillBook, BillBookTemplate, BillBookTemplateContent, Role};
use crate::model::request::bill::book::bill_book_edit_request::BillBookEditRequest;
use crate::model::request::bill::book::bill_book_request::BillBookRequest;
use crate::utils::database::get_connection;

pub fn get_bill_book_list(filter_name: Option<String>,login_user_info: &LoginUserInfo) -> Vec<BillBook> {
    let connection = config::connection("FORTUNE_DATABASE_URL".to_string());
    use crate::model::diesel::fortune::fortune_schema::bill_book as bill_book_table;
    let mut query = bill_book_table::table.into_boxed::<diesel::pg::Pg>();
    if let Some(some_filter_name) = &filter_name {
        query = query.filter(bill_book_table::name.like(format!("{}{}{}","%",some_filter_name.as_str(),"%")));
    }
    query = query.filter(bill_book_table::creator.eq(login_user_info.userId));
    let user_bill_books = query
        .load::<BillBook>(&connection)
        .expect("error get user bill book");
    return user_bill_books;
}

pub fn edit_bill_book(request: &Json<BillBookEditRequest>) -> BillBook {
    use crate::model::diesel::fortune::fortune_schema::bill_book::dsl::*;
    let predicate = id.eq(request.bill_book_id);
    let update_result = diesel::update(bill_book.filter(predicate))
        .set(name.eq(request.name.to_string()))
        .get_result(&get_connection());
    return update_result.unwrap();
}

pub fn get_bill_book_by_id(filter_bill_book_id: &i64) -> BillBook{
    let connection = config::connection("FORTUNE_DATABASE_URL".to_string());
    use crate::model::diesel::fortune::fortune_schema::bill_book::dsl::*;
    let predicate = id.eq(filter_bill_book_id);
    let templates = bill_book
        .filter(predicate)
        .load::<BillBook>(&connection)
        .expect("error get user contents");
    return templates.get(0).unwrap().to_owned();
}

fn get_template_list_by_id(template_id: i64) -> Vec<BillBookTemplate>{
    let connection = config::connection("FORTUNE_DATABASE_URL".to_string());
    use crate::model::diesel::fortune::fortune_schema::bill_book_template::dsl::*;
    let predicate = id.eq(template_id);
    let templates = bill_book_template
        .filter(predicate)
        .load::<BillBookTemplate>(&connection)
        .expect("error get user contents");
    return templates;
}

fn get_template_list_count_by_user_id(filter_user_id: &i64) -> i64{
    let connection = config::connection("FORTUNE_DATABASE_URL".to_string());
    use crate::model::diesel::fortune::fortune_schema::bill_book::dsl::*;
    let predicate = creator.eq(filter_user_id);
    let templates_count = bill_book
        .filter(predicate)
        .count()
        .get_result(&connection);
    return templates_count.unwrap_or(0);
}

///
/// 新增账本时，除了初始化账本数据
/// 还要初始化当前账本收入、支出等类型的目录数据
/// 不同的账本目录可自定义
///
pub fn add_bill_book(request:&Json<BillBookRequest>, login_user_info: &LoginUserInfo) -> Result<BillBook, String> {
    let connection = get_connection();
    let templates = get_template_list_by_id(request.billBookTemplateId);
    if templates.is_empty() {
        return Err("the template did not exists, check your template id first".parse().unwrap());
    }
    let templates_count = get_template_list_count_by_user_id(&login_user_info.userId);
    if templates_count >= 20 {
        return Err("2 bill book for every user".parse().unwrap());
    }
    let transaction_result = connection.build_transaction()
        .repeatable_read()
        .run::<_, diesel::result::Error, _>(||{
             return add_bill_book_impl(login_user_info, &templates, request);
        });
    return match transaction_result {
        Ok(v) => {
            Ok(v)
        },
        Err(_e) => {
            Err("database error".parse().unwrap())
        }
    };
}

///
/// 初始化账本数据
///
fn add_bill_book_impl(login_user_info: &LoginUserInfo, templates: &Vec<BillBookTemplate>, request:&Json<BillBookRequest>) -> Result<BillBook,diesel::result::Error>{
    let bill_book_record = BillBookAdd{
        created_time: get_current_millisecond(),
        updated_time: get_current_millisecond(),
        deleted: 0,
        creator: login_user_info.userId,
        name: templates.get(0).unwrap().to_owned().name,
        bill_book_template_id: request.billBookTemplateId
    };
    let inserted_record = diesel::insert_into(crate::model::diesel::fortune::fortune_schema::bill_book::table)
        .values(&bill_book_record)
        .on_conflict_do_nothing()
        .get_results::<BillBook>(&get_connection());
    let records = inserted_record.unwrap();
    // 使用to_owned()表示重新拷贝了一份数据，和重新构建一个String出来别无二致
    let new_bill_book = records.get(0).unwrap().to_owned();
    add_bill_book_categories(&new_bill_book, login_user_info);
    add_bill_book_role(&new_bill_book, login_user_info);
    add_bill_book_account(&new_bill_book, login_user_info);
    return Ok(new_bill_book);
}

///
/// 初始化账本目录数据
///
fn add_bill_book_categories(bill_book: &BillBook, login_user_info: &LoginUserInfo){
    let connection = config::connection("FORTUNE_DATABASE_URL".to_string());
    use crate::model::diesel::fortune::fortune_schema::bill_book_template_contents::dsl::*;
    let predicate = bill_book_template_id.eq(bill_book.bill_book_template_id);
    let categories_record = bill_book_template_contents
        .filter(predicate)
        .load::<BillBookTemplateContent>(&connection)
        .expect("error get categories contents");
    let mut bill_book_contents:Vec<BillBookContentAdd> = Vec::new();
    for record in categories_record {
        let bill_book_content = BillBookContentAdd{
            created_time: get_current_millisecond(),
            updated_time: get_current_millisecond(),
            deleted: 0,
            creator: login_user_info.userId,
            bill_book_id: bill_book.id,
            name: record.name,
            contents: "".to_string(),
            content_id: record.id,
            parent_id: record.parent_id,
            contents_type: record.contents_type
        };
        bill_book_contents.push(bill_book_content);
    }
    diesel::insert_into(crate::model::diesel::fortune::fortune_schema::bill_book_contents::table)
        .values(&bill_book_contents)
        .on_conflict_do_nothing()
        .execute(&connection)
        .unwrap();
}

///
/// 初始化账本角色数据
///
fn add_bill_book_role(bill_book: &BillBook, login_user_info: &LoginUserInfo){
    let connection = config::connection("FORTUNE_DATABASE_URL".to_string());
    use crate::model::diesel::fortune::fortune_schema::role::dsl::*;
    let predicate = role_type.eq(1);
    let categories_record = role
        .filter(predicate)
        .load::<Role>(&connection)
        .expect("error get categories contents");
    let mut bill_book_roles:Vec<BillBookRoleAdd> = Vec::new();
    for record in categories_record {
        let bill_book_content = BillBookRoleAdd{
            created_time: get_current_millisecond(),
            updated_time: get_current_millisecond(),
            deleted: 0,
            creator: login_user_info.userId,
            bill_book_id: bill_book.id,
            remark: Option::from(record.remark),
            name: record.name,
            role_type: record.role_type
        };
        bill_book_roles.push(bill_book_content);
    }
    diesel::insert_into(crate::model::diesel::fortune::fortune_schema::bill_book_role::table)
        .values(&bill_book_roles)
        .on_conflict_do_nothing()
        .execute(&connection)
        .unwrap();
}

///
/// 初始化账本账户类型数据
///
fn add_bill_book_account(bill_book: &BillBook, login_user_info: &LoginUserInfo){
    let connection = get_connection();
    use crate::model::diesel::fortune::fortune_schema::account::dsl::*;
    let categories_record = account
        .load::<Account>(&connection)
        .expect("error get categories contents");
    let mut bill_book_roles:Vec<BillBookAccountAdd> = Vec::new();
    for record in categories_record {
        let bill_book_content = BillBookAccountAdd{
            created_time: get_current_millisecond(),
            updated_time: get_current_millisecond(),
            deleted: 0,
            creator: login_user_info.userId,
            bill_book_id: bill_book.id,
            remark: "".parse().unwrap(),
            account_id: record.id,
            name: record.name,
        };
        bill_book_roles.push(bill_book_content);
    }
    diesel::insert_into(crate::model::diesel::fortune::fortune_schema::bill_book_account::table)
        .values(&bill_book_roles)
        .on_conflict_do_nothing()
        .execute(&connection)
        .unwrap();
}