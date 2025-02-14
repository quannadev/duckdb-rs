//! [`ToSql`] and [`FromSql`] implementation for JSON `Value`.

use serde_json::Value;

use crate::{
    types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef},
    Result,
};

/// Serialize JSON `Value` to text.
impl ToSql for Value {
    #[inline]
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(
            serde_json::to_string(self).unwrap().replace("null", "NULL"),
        ))
    }
}

/// Deserialize text/blob to JSON `Value`.
impl FromSql for Value {
    #[inline]
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Text(s) => serde_json::from_slice(s),
            ValueRef::Blob(b) => serde_json::from_slice(b),
            _ => return Err(FromSqlError::InvalidType),
        }
        .map_err(|err| FromSqlError::Other(Box::new(err)))
    }
}

#[cfg(test)]
mod test {
    use crate::{types::ToSql, Connection, Result};
    use serde_json::json;

    fn checked_memory_handle() -> Result<Connection> {
        let db = Connection::open_in_memory()?;
        db.execute_batch("CREATE TABLE foo (t TEXT, b BLOB)")?;
        Ok(db)
    }

    #[test]
    fn test_json_value() -> Result<()> {
        let db = checked_memory_handle()?;

        let json = r#"{"foo": 13, "bar": "baz"}"#;
        let data: serde_json::Value = serde_json::from_str(json).unwrap();
        db.execute(
            "INSERT INTO foo (t, b) VALUES (?, ?)",
            [&data as &dyn ToSql, &json.as_bytes()],
        )?;

        let t: serde_json::Value = db.query_row("SELECT t FROM foo", [], |r| r.get(0))?;
        assert_eq!(data, t);
        let b: serde_json::Value = db.query_row("SELECT b FROM foo", [], |r| r.get(0))?;
        assert_eq!(data, b);
        Ok(())
    }

    #[test]
    fn test_json_null_value() -> Result<()> {
        let db = checked_memory_handle()?;
        db.execute("INSERT INTO foo (n) VALUES (?)", [serde_json::Value::Null])?;
        let res: usize = db.query_row("SELECT COUNT(*) FROM foo", [], |r| r.get(0)).unwrap();
        assert_eq!(res, 1);
        db.execute("INSERT INTO foo (s) VALUES (?)", [serde_json::Value::Null])?;
        let res: usize = db.query_row("SELECT COUNT(*) FROM foo", [], |r| r.get(0)).unwrap();
        assert_eq!(res, 2);
        let json = json!({ "i": null });
        db.execute("INSERT INTO foo (s) VALUES (?)", [&json])?;
        let res: usize = db.query_row("SELECT COUNT(*) FROM foo", [], |r| r.get(0)).unwrap();
        assert_eq!(res, 3);
        let json = json!({
            "i": 1
        });
        db.execute("INSERT INTO foo (s) VALUES (?)", [&json])?;
        let mut query = db.prepare("SELECT s FROM foo").unwrap();
        let rows = query.query_arrow([]).unwrap().collect::<Vec<_>>();
        assert_eq!(rows.len(), 1);
        let first_batch = rows.first().unwrap();
        assert_eq!(first_batch.num_rows(), 4);
        Ok(())
    }
}
