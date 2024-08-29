use parse_datetime::parse_datetime;

fn main() {
    let date: String = std::env::args().nth(1).unwrap_or("".to_string());
    println!("{}", parse_datetime(&date).unwrap().format("%+"))
}
