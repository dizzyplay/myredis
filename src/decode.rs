use std::collections::VecDeque;
use bytes::BytesMut;

#[derive(Debug)]
pub enum Value {
    Array(Vec<Value>),
    String(String),
}
pub struct Decoder {
    data: VecDeque<String>,
}
impl Decoder {
    pub fn new(data: BytesMut) -> Decoder {
        let command = String::from_utf8(data.clone().to_vec()).unwrap();
        let command_list: VecDeque<String> = command.split("\r\n").map(|s| s.to_string()).collect();
        Decoder { data: command_list }
    }

    pub fn parse(&mut self) -> VecDeque<String> {
        let mut arr = VecDeque::new();

        while let Some(command) = self.data.pop_front() {
            match command.as_str() {
                cmd if cmd.starts_with('$') => {
                    // '$' 뒤 숫자를 파싱하여 문자열 길이로 사용
                    let string_length = cmd
                        .strip_prefix('$')
                        .ok_or("Expected '$' prefix").unwrap()
                        .parse::<usize>()
                        .map_err(|_| "Invalid string length after '$'").unwrap();

                    // 문자열 길이에 해당하는 데이터를 읽어오기
                    if let Some(data) = self.data.pop_front() {
                        if data.len() == string_length {
                            arr.push_back(data);
                        } else {
                            panic!("Expected string of length {}", string_length);
                        }
                    } else {
                        panic!("Expected string data after length");
                    }
                }
                _ => {
                    // 배열 외의 다른 데이터가 나오면 종료
                    continue;
                }
            }
        }
        arr
    }
}
