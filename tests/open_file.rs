#[cfg(test)]
mod tests {
    use dbase::open;
    use dbase::header::Record;
    #[test]
    fn parse_file() {
        let r = open("test31.dbf");
        let db = r.unwrap();
        db.into_iter().for_each(|i:Record| {
            println!("{:?}", i);
        });
    }
}