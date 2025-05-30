use crate::config;
use postgres::types::ToSql;
use sql_query_builder as sql;

pub type SqlQuery = (String, Vec<Box<dyn ToSql + Sync>>);

pub trait Ops {
    fn to_upsert_sql(&self) -> SqlQuery;
}

impl dyn Ops {
    pub fn last_column(table_name: &str, column_name: &str, desc: bool) -> SqlQuery {
        let sort_order = if desc { "desc" } else { "asc" };
        let select = sql::Select::new()
            .select(column_name)
            .from(table_name)
            .order_by(&format!("{} {}", column_name, sort_order))
            .limit("1");
        (select.as_string(), vec![])
    }

    pub fn upsert_sql(
        table_name: &str,
        column_index_names: Vec<&str>,
        column_other_names: Vec<&str>,
        column_values: Vec<Box<dyn ToSql + Sync>>,
    ) -> SqlQuery {
        let all_names = vec![column_index_names.clone(), column_other_names.clone()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let value_positions = (1..all_names.len() + 1)
            .map(|x| format!("${}", x))
            .collect::<Vec<_>>()
            .join(", ");
        let conflict = if column_index_names.len() > 0 {
            format!(
                "({}) DO UPDATE SET {}",
                column_index_names.join(", "),
                column_other_names
                    .into_iter()
                    .map(|x| format!("{} = EXCLUDED.{}", x, x))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        } else {
            format!("DO NOTHING")
        };
        let select = sql::Insert::new()
            .insert_into(&format!("{} ({})", table_name, all_names.join(", ")))
            .values(&format!("({})", value_positions))
            .on_conflict(&conflict);
        (select.as_string(), column_values)
    }
}

pub struct TransactionClient<'a> {
    pub client: postgres::Transaction<'a>,
    pub xact_id: String,
}

impl TransactionClient<'_> {
    pub fn new(db: &mut Client) -> TransactionClient<'_> {
        let mut transaction = db.client.transaction().unwrap();
        let row = transaction
            .query_one("select pg_current_xact_id()::text", &vec![])
            .unwrap();
        let xact_id = row.get::<&str, Option<String>>("pg_current_xact_id");
        TransactionClient {
            client: transaction,
            xact_id: xact_id.unwrap(),
        }
    }

    pub fn q(&mut self, query: SqlQuery) -> Vec<postgres::Row> {
        log::info!(target: "sql", "[xact {}] {} {:?}", self.xact_id, query.0, query.1);
        let params: Vec<&(dyn ToSql + Sync)> = query.1.iter().map(|y| &**y).collect();
        self.client.query(&query.0, &params).unwrap()
    }

    pub fn first(&mut self, query: SqlQuery) -> Option<postgres::Row> {
        let mut rows = self.q(query);
        if rows.len() > 0 {
            Some(rows.remove(0))
        } else {
            None
        }
    }
}

pub struct Client {
    pub client: postgres::Client,
}

pub(crate) fn new() -> Client {
    let config = config::CONFIG.get().unwrap();

    let mut client = postgres::Client::connect(&config.psql, postgres::NoTls).unwrap();

    log::info!("sql connected");
    embedded::migrations::runner().run(&mut client).unwrap();
    return Client { client };
}

impl Client {
    pub fn q_last(&mut self, query: SqlQuery) -> Option<postgres::Row> {
        let row = self.q(query);
        if row.len() > 0 {
            Some(row.into_iter().last().unwrap())
        } else {
            None
        }
    }

    pub fn q(&mut self, query: SqlQuery) -> Vec<postgres::Row> {
        log::info!(target: "sql", "{} {:?}", query.0, query.1);
        let params: Vec<&(dyn ToSql + Sync)> = query.1.iter().map(|y| &**y).collect();
        self.client.query(&query.0, &params).unwrap()
    }
}

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("./sql");
}
