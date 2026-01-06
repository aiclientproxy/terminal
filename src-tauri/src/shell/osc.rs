//! OSC 序列处理
//!
//! 处理 OSC 7 (工作目录) 和 OSC 52 (剪贴板) 等特殊序列。
//!
//! ## OSC 序列格式
//!
//! OSC (Operating System Command) 序列的格式为:
//! - `ESC ] Ps ; Pt BEL` 或 `ESC ] Ps ; Pt ST`
//! - ESC = 0x1B, BEL = 0x07, ST = ESC \
//!
//! ## 支持的序列
//!
//! - OSC 7: 工作目录通知 (`file://hostname/path`)
//! - OSC 52: 剪贴板操作 (`selection;base64_data`)

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

/// ESC 字符
const ESC: char = '\x1b';
/// BEL 字符 (终止符)
const BEL: char = '\x07';
/// OSC 起始序列
const OSC_START: &str = "\x1b]";
/// ST 终止序列 (ESC \)
const ST: &str = "\x1b\\";

/// OSC 序列类型
#[derive(Debug, Clone, PartialEq)]
pub enum OscSequence {
    /// OSC 7: 工作目录
    WorkingDirectory(String),
    /// OSC 52: 剪贴板内容
    Clipboard(ClipboardData),
    /// 未知或无效序列
    Unknown,
}

/// 剪贴板数据
#[derive(Debug, Clone, PartialEq)]
pub struct ClipboardData {
    /// 剪贴板选择类型 (c=clipboard, p=primary, q=secondary, s=select, 0-7=cut buffers)
    pub selection: ClipboardSelection,
    /// 解码后的内容
    pub content: String,
}

/// 剪贴板选择类型
#[derive(Debug, Clone, PartialEq)]
pub enum ClipboardSelection {
    /// 系统剪贴板 (c)
    Clipboard,
    /// Primary 选择 (p) - X11
    Primary,
    /// Secondary 选择 (q) - X11
    Secondary,
    /// Select 选择 (s) - X11
    Select,
    /// Cut buffer (0-7)
    CutBuffer(u8),
}

impl ClipboardSelection {
    /// 从字符解析选择类型
    fn from_char(c: char) -> Option<Self> {
        match c {
            'c' => Some(Self::Clipboard),
            'p' => Some(Self::Primary),
            'q' => Some(Self::Secondary),
            's' => Some(Self::Select),
            '0'..='7' => Some(Self::CutBuffer(c as u8 - b'0')),
            _ => None,
        }
    }
}

/// OSC 解析结果
#[derive(Debug, Clone)]
pub struct OscParseResult {
    /// 解析出的 OSC 序列
    pub sequence: OscSequence,
    /// 原始 OSC 序列在输入中的起始位置
    pub start: usize,
    /// 原始 OSC 序列在输入中的结束位置（不包含）
    pub end: usize,
}

/// OSC 处理器
///
/// 负责从终端输出流中检测和解析 OSC 序列。
pub struct OscHandler {
    /// 剪贴板数据大小限制 (字节)
    max_clipboard_size: usize,
}

impl OscHandler {
    /// 创建新的 OSC 处理器
    pub fn new() -> Self {
        Self {
            max_clipboard_size: 1024 * 1024, // 1MB
        }
    }

    /// 设置剪贴板大小限制
    pub fn with_max_clipboard_size(mut self, size: usize) -> Self {
        self.max_clipboard_size = size;
        self
    }

    /// 获取剪贴板大小限制
    pub fn max_clipboard_size(&self) -> usize {
        self.max_clipboard_size
    }

    /// 解析 OSC 序列内容
    ///
    /// 输入应该是去掉了 `ESC ]` 前缀和 `BEL`/`ST` 后缀的内容。
    ///
    /// # 示例
    ///
    /// ```
    /// use terminal_plugin::shell::osc::OscHandler;
    ///
    /// let handler = OscHandler::new();
    /// let result = handler.parse("7;file://localhost/home/user");
    /// ```
    pub fn parse(&self, data: &str) -> OscSequence {
        // 空数据返回 Unknown
        if data.is_empty() {
            return OscSequence::Unknown;
        }

        // OSC 7: 工作目录
        if let Some(rest) = data.strip_prefix("7;") {
            if let Some(path) = self.parse_file_url(rest) {
                return OscSequence::WorkingDirectory(path);
            }
            // 尝试直接解析路径（某些终端可能不使用 file:// 前缀）
            if rest.starts_with('/') {
                return OscSequence::WorkingDirectory(urlencoding_decode(rest));
            }
        }

        // OSC 52: 剪贴板
        if let Some(rest) = data.strip_prefix("52;") {
            if let Some(clipboard_data) = self.parse_clipboard(rest) {
                return OscSequence::Clipboard(clipboard_data);
            }
        }

        OscSequence::Unknown
    }

    /// 从原始终端输出中提取所有 OSC 序列
    ///
    /// 返回找到的所有 OSC 序列及其位置信息。
    pub fn extract_sequences(&self, data: &str) -> Vec<OscParseResult> {
        let mut results = Vec::new();
        let mut search_start = 0;

        while let Some(osc_start) = data[search_start..].find(OSC_START) {
            let absolute_start = search_start + osc_start;

            // 查找终止符 (BEL 或 ST)
            let content_start = absolute_start + OSC_START.len();
            if content_start >= data.len() {
                break;
            }

            let remaining = &data[content_start..];

            // 查找 BEL 终止符
            let bel_pos = remaining.find(BEL);
            // 查找 ST 终止符
            let st_pos = remaining.find(ST);

            // 选择最近的终止符
            let (end_offset, terminator_len) = match (bel_pos, st_pos) {
                (Some(b), Some(s)) => {
                    if b <= s {
                        (b, 1) // BEL 是单字符
                    } else {
                        (s, ST.len())
                    }
                }
                (Some(b), None) => (b, 1),
                (None, Some(s)) => (s, ST.len()),
                (None, None) => {
                    // 没有找到终止符，跳过这个 OSC 起始
                    search_start = content_start;
                    continue;
                }
            };

            let osc_content = &remaining[..end_offset];
            let absolute_end = content_start + end_offset + terminator_len;

            // 解析 OSC 内容
            let sequence = self.parse(osc_content);

            results.push(OscParseResult {
                sequence,
                start: absolute_start,
                end: absolute_end,
            });

            search_start = absolute_end;
        }

        results
    }

    /// 从原始终端输出中移除所有 OSC 序列
    ///
    /// 返回移除 OSC 序列后的数据和提取出的序列列表。
    pub fn strip_sequences(&self, data: &str) -> (String, Vec<OscSequence>) {
        let results = self.extract_sequences(data);

        if results.is_empty() {
            return (data.to_string(), Vec::new());
        }

        let mut stripped = String::with_capacity(data.len());
        let mut last_end = 0;

        let sequences: Vec<OscSequence> = results
            .iter()
            .map(|r| {
                // 添加 OSC 序列之前的内容
                stripped.push_str(&data[last_end..r.start]);
                last_end = r.end;
                r.sequence.clone()
            })
            .collect();

        // 添加最后一个 OSC 序列之后的内容
        stripped.push_str(&data[last_end..]);

        (stripped, sequences)
    }

    /// 解析 file:// URL
    fn parse_file_url(&self, url: &str) -> Option<String> {
        if let Some(rest) = url.strip_prefix("file://") {
            // 跳过主机名部分（可能为空或 localhost）
            if let Some(path_start) = rest.find('/') {
                let path = &rest[path_start..];
                // URL 解码
                return Some(urlencoding_decode(path));
            }
            // 如果没有找到路径分隔符，可能是 Windows 路径 (file:///C:/...)
            // 或者主机名后直接是空的
            if rest.is_empty() {
                return None;
            }
        }
        None
    }

    /// 解析剪贴板数据
    fn parse_clipboard(&self, data: &str) -> Option<ClipboardData> {
        // 格式: selection;base64_data
        // selection 可以是 c (clipboard), p (primary), 等
        let parts: Vec<&str> = data.splitn(2, ';').collect();
        if parts.len() != 2 {
            return None;
        }

        let selection_str = parts[0];
        let base64_data = parts[1];

        // 解析选择类型（取第一个字符）
        let selection = selection_str
            .chars()
            .next()
            .and_then(ClipboardSelection::from_char)
            .unwrap_or(ClipboardSelection::Clipboard);

        // 检查大小限制
        if base64_data.len() > self.max_clipboard_size {
            tracing::warn!(
                "剪贴板数据超过大小限制: {} > {}",
                base64_data.len(),
                self.max_clipboard_size
            );
            return None;
        }

        // 空数据是有效的（用于查询剪贴板）
        if base64_data.is_empty() {
            return Some(ClipboardData {
                selection,
                content: String::new(),
            });
        }

        // Base64 解码
        match BASE64.decode(base64_data) {
            Ok(bytes) => {
                match String::from_utf8(bytes) {
                    Ok(content) => Some(ClipboardData { selection, content }),
                    Err(_) => {
                        tracing::warn!("剪贴板数据不是有效的 UTF-8");
                        None
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Base64 解码失败: {}", e);
                None
            }
        }
    }
}

impl Default for OscHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// URL 解码
///
/// 将 URL 编码的字符串解码为原始字符串。
/// 支持 %XX 格式的编码。
pub fn urlencoding_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut bytes = s.as_bytes().iter().peekable();

    while let Some(&byte) = bytes.next() {
        if byte == b'%' {
            // 尝试读取两个十六进制字符
            let hex1 = bytes.next().copied();
            let hex2 = bytes.next().copied();

            if let (Some(h1), Some(h2)) = (hex1, hex2) {
                let hex_str = [h1, h2];
                if let Ok(hex_str) = std::str::from_utf8(&hex_str) {
                    if let Ok(decoded_byte) = u8::from_str_radix(hex_str, 16) {
                        result.push(decoded_byte as char);
                        continue;
                    }
                }
                // 解码失败，保留原始字符
                result.push('%');
                result.push(h1 as char);
                result.push(h2 as char);
            } else {
                // 不完整的编码，保留原始字符
                result.push('%');
                if let Some(h1) = hex1 {
                    result.push(h1 as char);
                }
            }
        } else {
            result.push(byte as char);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_osc7_working_directory() {
        let handler = OscHandler::new();
        let result = handler.parse("7;file://localhost/home/user/projects");
        assert_eq!(
            result,
            OscSequence::WorkingDirectory("/home/user/projects".to_string())
        );
    }

    #[test]
    fn test_parse_osc7_empty_hostname() {
        let handler = OscHandler::new();
        // 某些终端使用空主机名
        let result = handler.parse("7;file:///home/user/projects");
        assert_eq!(
            result,
            OscSequence::WorkingDirectory("/home/user/projects".to_string())
        );
    }

    #[test]
    fn test_parse_osc7_direct_path() {
        let handler = OscHandler::new();
        // 某些终端直接发送路径
        let result = handler.parse("7;/home/user/projects");
        assert_eq!(
            result,
            OscSequence::WorkingDirectory("/home/user/projects".to_string())
        );
    }

    #[test]
    fn test_parse_osc7_url_encoded() {
        let handler = OscHandler::new();
        let result = handler.parse("7;file://localhost/home/user/my%20project");
        assert_eq!(
            result,
            OscSequence::WorkingDirectory("/home/user/my project".to_string())
        );
    }

    #[test]
    fn test_parse_osc52_clipboard() {
        let handler = OscHandler::new();
        // "Hello" in base64 is "SGVsbG8="
        let result = handler.parse("52;c;SGVsbG8=");
        assert_eq!(
            result,
            OscSequence::Clipboard(ClipboardData {
                selection: ClipboardSelection::Clipboard,
                content: "Hello".to_string(),
            })
        );
    }

    #[test]
    fn test_parse_osc52_primary() {
        let handler = OscHandler::new();
        let result = handler.parse("52;p;SGVsbG8=");
        assert_eq!(
            result,
            OscSequence::Clipboard(ClipboardData {
                selection: ClipboardSelection::Primary,
                content: "Hello".to_string(),
            })
        );
    }

    #[test]
    fn test_parse_osc52_empty_content() {
        let handler = OscHandler::new();
        // 空内容用于查询剪贴板
        let result = handler.parse("52;c;");
        assert_eq!(
            result,
            OscSequence::Clipboard(ClipboardData {
                selection: ClipboardSelection::Clipboard,
                content: String::new(),
            })
        );
    }

    #[test]
    fn test_parse_invalid_osc() {
        let handler = OscHandler::new();
        let result = handler.parse("invalid");
        assert_eq!(result, OscSequence::Unknown);
    }

    #[test]
    fn test_parse_empty_data() {
        let handler = OscHandler::new();
        let result = handler.parse("");
        assert_eq!(result, OscSequence::Unknown);
    }

    #[test]
    fn test_clipboard_size_limit() {
        let handler = OscHandler::new().with_max_clipboard_size(10);
        // 超过限制的数据
        let large_data = "c;".to_string() + &"A".repeat(100);
        let result = handler.parse(&format!("52;{}", large_data));
        assert_eq!(result, OscSequence::Unknown);
    }

    #[test]
    fn test_clipboard_invalid_base64() {
        let handler = OscHandler::new();
        let result = handler.parse("52;c;not-valid-base64!!!");
        assert_eq!(result, OscSequence::Unknown);
    }

    #[test]
    fn test_url_decode() {
        assert_eq!(urlencoding_decode("/path/to/file"), "/path/to/file");
        assert_eq!(
            urlencoding_decode("/path%20with%20spaces"),
            "/path with spaces"
        );
        assert_eq!(urlencoding_decode("/path%2Fwith%2Fslashes"), "/path/with/slashes");
        assert_eq!(urlencoding_decode("%"), "%");
        assert_eq!(urlencoding_decode("%2"), "%2");
        assert_eq!(urlencoding_decode("%ZZ"), "%ZZ");
    }

    #[test]
    fn test_extract_sequences_single() {
        let handler = OscHandler::new();
        let data = "normal text\x1b]7;file://localhost/home/user\x07more text";
        let results = handler.extract_sequences(data);

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].sequence,
            OscSequence::WorkingDirectory("/home/user".to_string())
        );
        assert_eq!(results[0].start, 11);
    }

    #[test]
    fn test_extract_sequences_multiple() {
        let handler = OscHandler::new();
        let data = "\x1b]7;file://localhost/home\x07text\x1b]52;c;SGVsbG8=\x07end";
        let results = handler.extract_sequences(data);

        assert_eq!(results.len(), 2);
        assert_eq!(
            results[0].sequence,
            OscSequence::WorkingDirectory("/home".to_string())
        );
        assert_eq!(
            results[1].sequence,
            OscSequence::Clipboard(ClipboardData {
                selection: ClipboardSelection::Clipboard,
                content: "Hello".to_string(),
            })
        );
    }

    #[test]
    fn test_extract_sequences_st_terminator() {
        let handler = OscHandler::new();
        let data = "text\x1b]7;file://localhost/home/user\x1b\\more";
        let results = handler.extract_sequences(data);

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].sequence,
            OscSequence::WorkingDirectory("/home/user".to_string())
        );
    }

    #[test]
    fn test_extract_sequences_no_terminator() {
        let handler = OscHandler::new();
        let data = "text\x1b]7;file://localhost/home/user";
        let results = handler.extract_sequences(data);

        // 没有终止符，不应该提取
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_strip_sequences() {
        let handler = OscHandler::new();
        let data = "before\x1b]7;file://localhost/home\x07after";
        let (stripped, sequences) = handler.strip_sequences(data);

        assert_eq!(stripped, "beforeafter");
        assert_eq!(sequences.len(), 1);
        assert_eq!(
            sequences[0],
            OscSequence::WorkingDirectory("/home".to_string())
        );
    }

    #[test]
    fn test_strip_sequences_no_osc() {
        let handler = OscHandler::new();
        let data = "normal text without OSC";
        let (stripped, sequences) = handler.strip_sequences(data);

        assert_eq!(stripped, data);
        assert!(sequences.is_empty());
    }

    #[test]
    fn test_clipboard_selection_types() {
        assert_eq!(
            ClipboardSelection::from_char('c'),
            Some(ClipboardSelection::Clipboard)
        );
        assert_eq!(
            ClipboardSelection::from_char('p'),
            Some(ClipboardSelection::Primary)
        );
        assert_eq!(
            ClipboardSelection::from_char('q'),
            Some(ClipboardSelection::Secondary)
        );
        assert_eq!(
            ClipboardSelection::from_char('s'),
            Some(ClipboardSelection::Select)
        );
        assert_eq!(
            ClipboardSelection::from_char('0'),
            Some(ClipboardSelection::CutBuffer(0))
        );
        assert_eq!(
            ClipboardSelection::from_char('7'),
            Some(ClipboardSelection::CutBuffer(7))
        );
        assert_eq!(ClipboardSelection::from_char('x'), None);
    }
}


/// Property-based tests for OSC sequence handling
/// Feature: terminal-plugin, Property 4: OSC 序列处理健壮性
/// **验证: 需求 7.1, 7.2, 7.3**
#[cfg(test)]
mod proptests {
    use super::*;
    use base64::Engine;
    use proptest::prelude::*;

    // Strategy for generating valid Unix paths
    fn valid_path_strategy() -> impl Strategy<Value = String> {
        prop::collection::vec("[a-zA-Z0-9_.-]+", 1..5)
            .prop_map(|parts| format!("/{}", parts.join("/")))
    }

    // Strategy for generating valid hostnames
    fn hostname_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("localhost".to_string()),
            Just("".to_string()),
            "[a-z][a-z0-9-]{0,10}".prop_map(|s| s),
        ]
    }

    // Strategy for generating valid OSC 7 sequences (working directory)
    fn valid_osc7_strategy() -> impl Strategy<Value = (String, String)> {
        (hostname_strategy(), valid_path_strategy()).prop_map(|(host, path)| {
            let osc_content = format!("7;file://{}{}", host, path);
            (osc_content, path)
        })
    }

    // Strategy for generating valid clipboard content
    fn clipboard_content_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 !@#$%^&*()_+-=]{0,100}"
    }

    // Strategy for generating clipboard selection types
    fn clipboard_selection_strategy() -> impl Strategy<Value = char> {
        prop_oneof![
            Just('c'),
            Just('p'),
            Just('q'),
            Just('s'),
            (0u8..8).prop_map(|n| (b'0' + n) as char),
        ]
    }

    // Strategy for generating valid OSC 52 sequences (clipboard)
    fn valid_osc52_strategy() -> impl Strategy<Value = (String, String, char)> {
        (clipboard_selection_strategy(), clipboard_content_strategy()).prop_map(
            |(selection, content)| {
                let encoded = BASE64.encode(content.as_bytes());
                let osc_content = format!("52;{};{}", selection, encoded);
                (osc_content, content, selection)
            },
        )
    }

    // Strategy for generating arbitrary (potentially invalid) strings
    fn arbitrary_string_strategy() -> impl Strategy<Value = String> {
        prop::collection::vec(any::<u8>(), 0..200).prop_map(|bytes| {
            // Convert to string, replacing invalid UTF-8 with replacement char
            String::from_utf8_lossy(&bytes).to_string()
        })
    }

    // Strategy for generating raw terminal output with embedded OSC sequences
    fn terminal_output_with_osc_strategy() -> impl Strategy<Value = (String, Vec<String>)> {
        let normal_text = "[a-zA-Z0-9 ]{0,50}";
        let osc_path = valid_path_strategy();

        (
            normal_text.clone(),
            osc_path,
            normal_text.clone(),
            normal_text,
        )
            .prop_map(|(before, path, middle, after)| {
                let osc_seq = format!("\x1b]7;file://localhost{}\x07", path);
                let full_output = format!("{}{}{}{}", before, osc_seq, middle, after);
                (full_output, vec![path])
            })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: terminal-plugin, Property 4: OSC 序列处理健壮性
        /// *对于任意*有效的 OSC 7 序列，正确解析出目录路径
        #[test]
        fn prop_osc7_parses_valid_paths((osc_content, expected_path) in valid_osc7_strategy()) {
            let handler = OscHandler::new();
            let result = handler.parse(&osc_content);

            match result {
                OscSequence::WorkingDirectory(path) => {
                    prop_assert_eq!(
                        path, expected_path,
                        "OSC 7 should parse to the expected path"
                    );
                }
                other => {
                    prop_assert!(
                        false,
                        "Expected WorkingDirectory, got {:?} for input: {}",
                        other, osc_content
                    );
                }
            }
        }

        /// Feature: terminal-plugin, Property 4: OSC 序列处理健壮性
        /// *对于任意*有效的 OSC 52 序列，正确解码 base64 内容
        #[test]
        fn prop_osc52_parses_valid_clipboard((osc_content, expected_content, selection_char) in valid_osc52_strategy()) {
            let handler = OscHandler::new();
            let result = handler.parse(&osc_content);

            match result {
                OscSequence::Clipboard(data) => {
                    prop_assert_eq!(
                        data.content, expected_content,
                        "OSC 52 should decode to the expected content"
                    );
                    // Verify selection type matches
                    let expected_selection = ClipboardSelection::from_char(selection_char)
                        .unwrap_or(ClipboardSelection::Clipboard);
                    prop_assert_eq!(
                        data.selection, expected_selection,
                        "OSC 52 should have the correct selection type"
                    );
                }
                other => {
                    prop_assert!(
                        false,
                        "Expected Clipboard, got {:?} for input: {}",
                        other, osc_content
                    );
                }
            }
        }

        /// Feature: terminal-plugin, Property 4: OSC 序列处理健壮性
        /// *对于任意*输入（包括无效的），OSC 处理器不应该崩溃
        #[test]
        fn prop_osc_never_panics(input in arbitrary_string_strategy()) {
            let handler = OscHandler::new();

            // This should never panic, regardless of input
            let result = std::panic::catch_unwind(|| {
                handler.parse(&input)
            });

            prop_assert!(
                result.is_ok(),
                "OscHandler::parse should never panic, but panicked on input: {:?}",
                input
            );
        }

        /// Feature: terminal-plugin, Property 4: OSC 序列处理健壮性
        /// *对于任意*无效的 OSC 序列，应该返回 Unknown
        #[test]
        fn prop_invalid_osc_returns_unknown(input in "[^0-9;][a-zA-Z0-9]{0,50}") {
            let handler = OscHandler::new();
            let result = handler.parse(&input);

            // Invalid sequences should return Unknown
            prop_assert_eq!(
                result,
                OscSequence::Unknown,
                "Invalid OSC sequence should return Unknown"
            );
        }

        /// Feature: terminal-plugin, Property 4: OSC 序列处理健壮性
        /// *对于任意*终端输出，extract_sequences 不应该崩溃
        #[test]
        fn prop_extract_sequences_never_panics(input in arbitrary_string_strategy()) {
            let handler = OscHandler::new();

            let result = std::panic::catch_unwind(|| {
                handler.extract_sequences(&input)
            });

            prop_assert!(
                result.is_ok(),
                "extract_sequences should never panic"
            );
        }

        /// Feature: terminal-plugin, Property 4: OSC 序列处理健壮性
        /// *对于任意*包含有效 OSC 序列的终端输出，应该正确提取序列
        #[test]
        fn prop_extract_sequences_finds_valid_osc((output, expected_paths) in terminal_output_with_osc_strategy()) {
            let handler = OscHandler::new();
            let results = handler.extract_sequences(&output);

            prop_assert_eq!(
                results.len(),
                expected_paths.len(),
                "Should find the expected number of OSC sequences"
            );

            for (result, expected_path) in results.iter().zip(expected_paths.iter()) {
                match &result.sequence {
                    OscSequence::WorkingDirectory(path) => {
                        prop_assert_eq!(
                            path, expected_path,
                            "Extracted path should match expected"
                        );
                    }
                    other => {
                        prop_assert!(
                            false,
                            "Expected WorkingDirectory, got {:?}",
                            other
                        );
                    }
                }
            }
        }

        /// Feature: terminal-plugin, Property 4: OSC 序列处理健壮性
        /// *对于任意*终端输出，strip_sequences 应该移除所有 OSC 序列
        #[test]
        fn prop_strip_sequences_removes_osc((output, _) in terminal_output_with_osc_strategy()) {
            let handler = OscHandler::new();
            let (stripped, sequences) = handler.strip_sequences(&output);

            // Stripped output should not contain OSC start sequence
            prop_assert!(
                !stripped.contains("\x1b]"),
                "Stripped output should not contain OSC start sequence"
            );

            // Should have extracted at least one sequence
            prop_assert!(
                !sequences.is_empty(),
                "Should have extracted at least one sequence"
            );
        }

        /// Feature: terminal-plugin, Property 4: OSC 序列处理健壮性
        /// *对于任意*剪贴板大小限制，超过限制的数据应该被拒绝
        #[test]
        fn prop_clipboard_size_limit_enforced(
            limit in 10usize..100,
            content_size in 1usize..200
        ) {
            let handler = OscHandler::new().with_max_clipboard_size(limit);

            // Generate content of specified size
            let content: String = "A".repeat(content_size);
            let encoded = BASE64.encode(content.as_bytes());
            let osc_content = format!("52;c;{}", encoded);

            let result = handler.parse(&osc_content);

            if encoded.len() > limit {
                // Should be rejected
                prop_assert_eq!(
                    result,
                    OscSequence::Unknown,
                    "Content exceeding size limit should be rejected"
                );
            } else {
                // Should be accepted
                match result {
                    OscSequence::Clipboard(data) => {
                        prop_assert_eq!(
                            data.content, content,
                            "Content within limit should be accepted"
                        );
                    }
                    _ => {
                        prop_assert!(
                            false,
                            "Content within limit should parse successfully"
                        );
                    }
                }
            }
        }

        /// Feature: terminal-plugin, Property 4: OSC 序列处理健壮性
        /// URL 解码往返测试：编码后解码应该得到原始字符串
        #[test]
        fn prop_url_decode_handles_encoded_chars(path in valid_path_strategy()) {
            // URL encode the path
            let encoded: String = path.chars().map(|c| {
                if c.is_ascii_alphanumeric() || c == '/' || c == '-' || c == '_' || c == '.' {
                    c.to_string()
                } else {
                    format!("%{:02X}", c as u8)
                }
            }).collect();

            let decoded = urlencoding_decode(&encoded);

            prop_assert_eq!(
                decoded, path,
                "URL decoding should recover the original path"
            );
        }
    }
}
