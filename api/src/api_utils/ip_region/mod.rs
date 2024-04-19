const CHINA: &str = "中国";
const INTERNAL_IP: &str = "内网IP";
const ZERO: &str = "0";
pub fn parse_region(region: &str) -> Option<String> {
    let split: Vec<&str> = region.split('|').collect();
    match split.as_slice() {
        [CHINA, _, province, _, _] if province.is_empty() || *province == ZERO => {
            Some(String::from(CHINA))
        }
        [CHINA, _, province, _, _] => Some(province.to_string()),
        [country, _, _, _, _] if *country != CHINA && *country != ZERO => Some(country.to_string()),
        [_, _, _, INTERNAL_IP, _] => Some(String::from(INTERNAL_IP)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_multi_type_ip() {
        let example1 = "美国|0|新墨西哥|0|康卡斯特";
        let example2 = "中国|0|福建|福州市|电信";
        let example3 = "0|0|0|内网IP|内网IP";
        println!("{:?}", parse_region(example1));
        println!("{:?}", parse_region(example2));
        println!("{:?}", parse_region(example3));
    }
}
