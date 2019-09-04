#[cfg(test)]
mod tests {
    use dbase_parser::fields::FieldValue;
    use dbase_parser::open;
    use chrono::{Utc};
    use chrono::offset::TimeZone;
    use dbase_parser::header::Record;

    #[test]
    fn parse_file_gis() {
        let db = open("tests/reference_gis.dbf").expect("Could not open file");
        let record = db.into_iter().next().expect("No first record");
        assert_eq!(record.get("NAME_4").unwrap(), &FieldValue::Text("Clervaux".to_string()));
        assert_eq!(record.get("NAME_3").unwrap(), &FieldValue::Text("Clervaux".to_string()));
        assert_eq!(record.get("NAME_2").unwrap(), &FieldValue::Text("Clervaux".to_string()));
        assert_eq!(record.get("NAME_1").unwrap(), &FieldValue::Text("Diekirch".to_string()));
        assert_eq!(record.get("NAME_0").unwrap(), &FieldValue::Text("Luxembourg".to_string()));
        assert_eq!(record.get("ISO").unwrap(), &FieldValue::Text("LUX".to_string()));
        assert_eq!(record.get("ID_4").unwrap(), &FieldValue::Numeric(1.0));
        assert_eq!(record.get("ID_3").unwrap(), &FieldValue::Numeric(1.0));
        assert_eq!(record.get("ID_2").unwrap(), &FieldValue::Numeric(1.0));
        assert_eq!(record.get("ID_1").unwrap(), &FieldValue::Numeric(1.0));
        assert_eq!(record.get("ID_0").unwrap(), &FieldValue::Numeric(131.0));
    }
    #[test]
    fn parse_file_fpt() {
        let r = open("tests/reference_fpt.dbf");
        let db = r.unwrap();
        
        let mut record_iter = db.into_iter();
        let record = record_iter.next().expect("Expected one record");
            assert_eq!(record.get("ID").unwrap(), &FieldValue::Numeric(20.0));
            assert_eq!(record.get("Name").unwrap(), &FieldValue::Text("srenauld".to_string()));
            assert_eq!(record.get("created_at").unwrap(), &FieldValue::DateTime(Utc.ymd(2019, 09, 04).and_hms(11, 6, 0)));
            assert_eq!(record.get("join").unwrap(), &FieldValue::Date(Utc.ymd(1999, 09, 03)));
            assert_eq!(record.get("active").unwrap(), &FieldValue::Boolean(Some(true)));
            assert_eq!(record.get("transfers").expect("No transfers"), &FieldValue::Integer(5));
            // assert_eq!(record.get("notes").expect("No notes"), &FieldValue::Text("This is a note.".to_string()));
        let record2 = record_iter.next().expect("Expected two records");
            assert_eq!(record2.get("ID").unwrap(), &FieldValue::Numeric(34.0));
            assert_eq!(record2.get("Name").unwrap(), &FieldValue::Text("Another".to_string()));
            assert_eq!(record2.get("created_at").unwrap(), &FieldValue::DateTime(Utc.ymd(2019, 09, 04).and_hms(11, 40, 0)));
            assert_eq!(record2.get("join").unwrap(), &FieldValue::Date(Utc.ymd(2019, 09, 04)));
            assert_eq!(record2.get("active").unwrap(), &FieldValue::Boolean(Some(false)));
            assert_eq!(record2.get("transfers").expect("No transfers"), &FieldValue::Integer(3));
            assert_eq!(record2.get("notes").expect("No notes"), &FieldValue::Text("This is a note.".to_string()));
    }
    #[test]
    fn parse_file_dpt() {
        let r = open("tests/reference_dbase.dbf");
        let db = r.unwrap();
        
        let mut record_iter = db.into_iter();
        let record = record_iter.next().expect("Expected one record in dbase III");
            assert_eq!(record.get("ID").unwrap(), &FieldValue::Numeric(87.0));
            assert_eq!(record.get("DESC").expect("No notes"), &FieldValue::Text("Our Original assortment...a little taste of heaven for everyone.  Let us
select a special assortment of our chocolate and pastel favorites for you.
Each petit four is its own special hand decorated creation. Multi-layers of
moist cake with combinations of specialty fillings create memorable cake
confections. Varietes include; Luscious Lemon, Strawberry Hearts, White
Chocolate, Mocha Bean, Roasted Almond, Triple Chocolate, Chocolate Hazelnut,
Grand Orange, Plum Squares, Milk chocolate squares, and Raspberry Blanc.".to_string().replace("\n", "\r\n")));
    }
}