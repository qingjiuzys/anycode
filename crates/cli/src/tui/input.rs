//! 多行输入缓冲、`^R` 反向搜索与历史导航。

use crate::md_tui::{text_display_width, wrap_string_to_width};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use super::styles::{style_brand, style_dim};
use super::util::{trim_or_default, truncate_preview};

include!("input_body.inc");
