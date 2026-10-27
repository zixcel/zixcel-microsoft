use crate::{Result, mailbox};
use serde_json::json;
pub struct PlainMail<'a> {
    pub to: &'a str,
    pub subject: &'a str,
    pub text: &'a str,
    pub reply_to: Option<&'a str>,
}
impl PlainMail<'_> {
    pub(crate) fn encode(self) -> Result<String> {
        if !mailbox(self.to)
            || self.reply_to.is_some_and(|v| !mailbox(v))
            || self.subject.trim().is_empty()
            || self.subject.len() > 998
            || self.subject.chars().any(char::is_control)
            || self.text.trim().is_empty()
            || self.text.len() > 32768
            || self
                .text
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\t' | '\r' | '\n'))
        {
            return Err("invalid_graph_mail");
        }
        let mut message = json!({"subject":self.subject,"body":{"contentType":"Text","content":self.text},"toRecipients":[{"emailAddress":{"address":self.to}}]});
        if let Some(reply) = self.reply_to {
            message["replyTo"] = json!([{"emailAddress":{"address":reply}}]);
        }
        Ok(json!({"message":message,"saveToSentItems":false}).to_string())
    }
}
