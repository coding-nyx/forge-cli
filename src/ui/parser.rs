use lazy_regex::lazy_regex;

#[derive(Debug, PartialEq)]
pub enum ResponsePart {
    Text(String),
    Shell(String),
}

pub struct ResponseParser;

impl ResponseParser {
    pub fn parse(input: &str) -> Vec<ResponsePart> {
        let mut parts = Vec::new();
        // Simplified regex without backreferences for compatibility
        let re = lazy_regex!(r"<(shell)>(.*?)</shell>");
        
        let mut last_end = 0;
        for cap in re.captures_iter(input) {
            let full_match = cap.get(0).unwrap();
            let content = cap.get(2).unwrap().as_str();
            
            // Add preceding text
            if full_match.start() > last_end {
                parts.push(ResponsePart::Text(input[last_end..full_match.start()].to_string()));
            }
            
            // Add the action
            parts.push(ResponsePart::Shell(content.trim().to_string()));
            last_end = full_match.end();
        }
        
        // Add remaining text
        if last_end < input.len() {
            parts.push(ResponsePart::Text(input[last_end..].to_string()));
        }
        
        parts
    }
}
