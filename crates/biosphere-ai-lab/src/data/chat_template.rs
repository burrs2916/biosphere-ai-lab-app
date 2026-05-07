use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::data::tokenizer::{BatchEncoding, TokenizerPipeline};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub messages: Vec<Message>,
    pub metadata: Option<HashMap<String, String>>,
}

impl Conversation {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            metadata: None,
        }
    }

    pub fn from_messages(messages: Vec<Message>) -> Self {
        Self {
            messages,
            metadata: None,
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push(Message {
            role: role.to_string(),
            content: content.to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        });
    }

    pub fn system(&mut self, content: &str) {
        self.add_message("system", content);
    }

    pub fn user(&mut self, content: &str) {
        self.add_message("user", content);
    }

    pub fn assistant(&mut self, content: &str) {
        self.add_message("assistant", content);
    }

    pub fn tool(&mut self, content: &str, tool_call_id: &str) {
        self.messages.push(Message {
            role: "tool".to_string(),
            content: content.to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: Some(tool_call_id.to_string()),
        });
    }

    pub fn last_message(&self) -> Option<&Message> {
        self.messages.last()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }
}

impl Default for Conversation {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub name: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub call_type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatTemplate {
    pub name: String,
    pub template_str: String,
    pub bos_token: Option<String>,
    pub eos_token: Option<String>,
    pub system_prompt: Option<String>,
    pub role_mapping: HashMap<String, String>,
    pub default_system_message: Option<String>,
}

impl ChatTemplate {
    pub fn new(name: &str, template_str: &str) -> Self {
        Self {
            name: name.to_string(),
            template_str: template_str.to_string(),
            bos_token: None,
            eos_token: None,
            system_prompt: None,
            role_mapping: HashMap::new(),
            default_system_message: None,
        }
    }

    pub fn llama3() -> Self {
        let mut role_mapping = HashMap::new();
        role_mapping.insert("user".to_string(), "user".to_string());
        role_mapping.insert("assistant".to_string(), "assistant".to_string());
        role_mapping.insert("system".to_string(), "system".to_string());
        role_mapping.insert("tool".to_string(), "tool".to_string());

        Self {
            name: "llama3".to_string(),
            template_str: "{{ bos_token }}{% for message in messages %}{% if message['role'] == 'system' %}<|start_header_id|>system<|end_header_id|>\n\n{{ message['content'] }}<|eot_id|>{% elif message['role'] == 'user' %}<|start_header_id|>user<|end_header_id|>\n\n{{ message['content'] }}<|eot_id|>{% elif message['role'] == 'assistant' %}<|start_header_id|>assistant<|end_header_id|>\n\n{{ message['content'] }}<|eot_id|>{% elif message['role'] == 'tool' %}<|start_header_id|>tool<|end_header_id|>\n\n{{ message['content'] }}<|eot_id|>{% endif %}{% endfor %}{% if add_generation_prompt %}<|start_header_id|>assistant<|end_header_id|>\n\n{% endif %}".to_string(),
            bos_token: Some("<|begin_of_text|>".to_string()),
            eos_token: Some("<|eot_id|>".to_string()),
            system_prompt: None,
            role_mapping,
            default_system_message: None,
        }
    }

    pub fn chatml() -> Self {
        let mut role_mapping = HashMap::new();
        role_mapping.insert("user".to_string(), "user".to_string());
        role_mapping.insert("assistant".to_string(), "assistant".to_string());
        role_mapping.insert("system".to_string(), "system".to_string());

        Self {
            name: "chatml".to_string(),
            template_str: "{% for message in messages %}<|im_start|>{{ message['role'] }}\n{{ message['content'] }}<|im_end|>\n{% endfor %}{% if add_generation_prompt %}<|im_start|>assistant\n{% endif %}".to_string(),
            bos_token: None,
            eos_token: Some("<|im_end|>".to_string()),
            system_prompt: None,
            role_mapping,
            default_system_message: Some("You are a helpful assistant.".to_string()),
        }
    }

    pub fn mistral() -> Self {
        let mut role_mapping = HashMap::new();
        role_mapping.insert("user".to_string(), "user".to_string());
        role_mapping.insert("assistant".to_string(), "assistant".to_string());
        role_mapping.insert("system".to_string(), "system".to_string());

        Self {
            name: "mistral".to_string(),
            template_str: "{{ bos_token }}{% for message in messages %}{% if message['role'] == 'system' %}[INST] {{ message['content'] }} [/INST]{% elif message['role'] == 'user' %}[INST] {{ message['content'] }} [/INST]{% elif message['role'] == 'assistant' %}{{ message['content'] }}{{ eos_token }}{% endif %}{% endfor %}".to_string(),
            bos_token: Some("<s>".to_string()),
            eos_token: Some("</s>".to_string()),
            system_prompt: None,
            role_mapping,
            default_system_message: None,
        }
    }

    pub fn zephyr() -> Self {
        let mut role_mapping = HashMap::new();
        role_mapping.insert("user".to_string(), "user".to_string());
        role_mapping.insert("assistant".to_string(), "assistant".to_string());
        role_mapping.insert("system".to_string(), "system".to_string());

        Self {
            name: "zephyr".to_string(),
            template_str: "{% for message in messages %}{% if message['role'] == 'system' %}<|system|>\n{{ message['content'] }}</s>\n{% elif message['role'] == 'user' %}<|user|>\n{{ message['content'] }}</s>\n{% elif message['role'] == 'assistant' %}<|assistant|>\n{{ message['content'] }}</s>\n{% endif %}{% endfor %}{% if add_generation_prompt %}<|assistant|>\n{% endif %}".to_string(),
            bos_token: None,
            eos_token: Some("</s>".to_string()),
            system_prompt: None,
            role_mapping,
            default_system_message: None,
        }
    }

    pub fn phi3() -> Self {
        let mut role_mapping = HashMap::new();
        role_mapping.insert("user".to_string(), "user".to_string());
        role_mapping.insert("assistant".to_string(), "assistant".to_string());
        role_mapping.insert("system".to_string(), "system".to_string());

        Self {
            name: "phi3".to_string(),
            template_str: "{{ bos_token }}{% for message in messages %}{% if message['role'] == 'system' %}<|system|>\n{{ message['content'] }}<|end|>\n{% elif message['role'] == 'user' %}<|user|>\n{{ message['content'] }}<|end|>\n{% elif message['role'] == 'assistant' %}<|assistant|>\n{{ message['content'] }}<|end|>\n{% endif %}{% endfor %}{% if add_generation_prompt %}<|assistant|>\n{% endif %}".to_string(),
            bos_token: Some("<s>".to_string()),
            eos_token: Some("<|end|>".to_string()),
            system_prompt: None,
            role_mapping,
            default_system_message: None,
        }
    }

    pub fn gemma() -> Self {
        let mut role_mapping = HashMap::new();
        role_mapping.insert("user".to_string(), "user".to_string());
        role_mapping.insert("assistant".to_string(), "model".to_string());
        role_mapping.insert("system".to_string(), "user".to_string());

        Self {
            name: "gemma".to_string(),
            template_str: "{{ bos_token }}{% for message in messages %}{% if message['role'] == 'user' or message['role'] == 'system' %}<start_of_turn>user\n{{ message['content'] }}<end_of_turn>\n{% elif message['role'] == 'assistant' %}<start_of_turn>model\n{{ message['content'] }}<end_of_turn>\n{% endif %}{% endfor %}{% if add_generation_prompt %}<start_of_turn>model\n{% endif %}".to_string(),
            bos_token: Some("<bos>".to_string()),
            eos_token: Some("<eos>".to_string()),
            system_prompt: None,
            role_mapping,
            default_system_message: None,
        }
    }

    pub fn apply(
        &self,
        conversation: &Conversation,
        add_generation_prompt: bool,
    ) -> Result<String, String> {
        let mut result = String::new();

        if let Some(ref bos) = self.bos_token {
            result.push_str(bos);
        }

        let has_system = conversation.messages.iter().any(|m| m.role == "system");

        if !has_system {
            if let Some(ref default_sys) = self.default_system_message {
                let mapped_role = self.role_mapping.get("system")
                    .cloned()
                    .unwrap_or_else(|| "system".to_string());
                result.push_str(&self.format_message(&mapped_role, default_sys));
            }
        }

        for message in &conversation.messages {
            let mapped_role = self.role_mapping.get(&message.role)
                .cloned()
                .unwrap_or_else(|| message.role.clone());
            result.push_str(&self.format_message(&mapped_role, &message.content));
        }

        if add_generation_prompt {
            let assistant_role = self.role_mapping.get("assistant")
                .cloned()
                .unwrap_or_else(|| "assistant".to_string());
            result.push_str(&self.format_generation_prompt(&assistant_role));
        }

        Ok(result)
    }

    fn format_message(&self, role: &str, content: &str) -> String {
        match self.name.as_str() {
            "llama3" => {
                format!("<|start_header_id|>{}<|end_header_id|>\n\n{}<|eot_id|>", role, content)
            }
            "chatml" => {
                format!("<|im_start|>{}\n{}<|im_end|>\n", role, content)
            }
            "mistral" => {
                if role == "user" || role == "system" {
                    format!("[INST] {} [/INST]", content)
                } else {
                    format!("{}{}", content, self.eos_token.as_deref().unwrap_or("</s>"))
                }
            }
            "zephyr" => {
                format!("<|{}|>\n{}</s>\n", role, content)
            }
            "phi3" => {
                format!("<|{}|>\n{}<|end|>\n", role, content)
            }
            "gemma" => {
                format!("<start_of_turn>{}\n{}<end_of_turn>\n", role, content)
            }
            _ => {
                format!("{}: {}\n", role, content)
            }
        }
    }

    fn format_generation_prompt(&self, role: &str) -> String {
        match self.name.as_str() {
            "llama3" => {
                format!("<|start_header_id|>{}<|end_header_id|>\n\n", role)
            }
            "chatml" => {
                format!("<|im_start|>{}\n", role)
            }
            "mistral" => {
                String::new()
            }
            "zephyr" => {
                format!("<|{}|>\n", role)
            }
            "phi3" => {
                format!("<|{}|>\n", role)
            }
            "gemma" => {
                format!("<start_of_turn>{}\n", role)
            }
            _ => {
                format!("{}: ", role)
            }
        }
    }

    pub fn tokenize_conversation(
        &self,
        tokenizer: &TokenizerPipeline,
        conversation: &Conversation,
        add_generation_prompt: bool,
    ) -> Result<BatchEncoding, String> {
        let text = self.apply(conversation, add_generation_prompt)?;
        tokenizer.encode(&text)
    }

    pub fn tokenize_conversations(
        &self,
        tokenizer: &TokenizerPipeline,
        conversations: &[Conversation],
        add_generation_prompt: bool,
    ) -> Result<BatchEncoding, String> {
        let texts: Result<Vec<String>, String> = conversations.iter()
            .map(|conv| self.apply(conv, add_generation_prompt))
            .collect();
        let texts = texts?;
        tokenizer.encode_batch(&texts, None)
    }
}

pub fn create_template(name: &str) -> Option<ChatTemplate> {
    match name.to_lowercase().as_str() {
        "llama3" | "llama-3" | "llama3.1" | "llama3.2" => Some(ChatTemplate::llama3()),
        "chatml" | "qwen" | "qwen2" | "qwen2.5" => Some(ChatTemplate::chatml()),
        "mistral" | "mistral-v0.1" | "mistral-v0.3" => Some(ChatTemplate::mistral()),
        "zephyr" | "zephyr-beta" => Some(ChatTemplate::zephyr()),
        "phi3" | "phi-3" | "phi3.5" => Some(ChatTemplate::phi3()),
        "gemma" | "gemma-2" | "gemma2" => Some(ChatTemplate::gemma()),
        _ => None,
    }
}

pub fn list_available_templates() -> Vec<ChatTemplateInfo> {
    vec![
        ChatTemplateInfo {
            name: "llama3".to_string(),
            description: "Llama 3/3.1/3.2 chat template".to_string(),
            supported_models: vec!["llama3".to_string(), "llama3.1".to_string(), "llama3.2".to_string()],
        },
        ChatTemplateInfo {
            name: "chatml".to_string(),
            description: "ChatML format (Qwen, Yi, DeepSeek)".to_string(),
            supported_models: vec!["qwen2".to_string(), "qwen2.5".to_string(), "yi".to_string(), "deepseek".to_string()],
        },
        ChatTemplateInfo {
            name: "mistral".to_string(),
            description: "Mistral instruct template".to_string(),
            supported_models: vec!["mistral".to_string(), "mixtral".to_string()],
        },
        ChatTemplateInfo {
            name: "zephyr".to_string(),
            description: "Zephyr H4 template".to_string(),
            supported_models: vec!["zephyr".to_string()],
        },
        ChatTemplateInfo {
            name: "phi3".to_string(),
            description: "Phi-3/3.5 chat template".to_string(),
            supported_models: vec!["phi3".to_string(), "phi3.5".to_string()],
        },
        ChatTemplateInfo {
            name: "gemma".to_string(),
            description: "Gemma/Gemma2 chat template".to_string(),
            supported_models: vec!["gemma".to_string(), "gemma2".to_string()],
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatTemplateInfo {
    pub name: String,
    pub description: String,
    pub supported_models: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationDataset {
    pub conversations: Vec<Conversation>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ConversationDataset {
    pub fn from_sharegpt_format(json_str: &str) -> Result<Self, String> {
        let data: Vec<ShareGptEntry> = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse ShareGPT format: {}", e))?;

        let conversations: Vec<Conversation> = data.into_iter()
            .map(|entry| {
                let messages: Vec<Message> = entry.conversations.into_iter()
                    .map(|sg_msg| Message {
                        role: sg_msg.from,
                        content: sg_msg.value,
                        name: None,
                        tool_calls: None,
                        tool_call_id: None,
                    })
                    .collect();
                Conversation {
                    messages,
                    metadata: None,
                }
            })
            .collect();

        Ok(Self {
            conversations,
            metadata: HashMap::new(),
        })
    }

    pub fn from_openai_format(json_str: &str) -> Result<Self, String> {
        #[derive(Deserialize)]
        struct OpenAiEntry {
            messages: Vec<OpenAiMessage>,
        }

        #[derive(Deserialize)]
        struct OpenAiMessage {
            role: String,
            content: String,
            #[serde(default)]
            name: Option<String>,
            #[serde(default)]
            tool_calls: Option<Vec<serde_json::Value>>,
            #[serde(default)]
            tool_call_id: Option<String>,
        }

        let data: Vec<OpenAiEntry> = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse OpenAI format: {}", e))?;

        let conversations: Vec<Conversation> = data.into_iter()
            .map(|entry| {
                let messages: Vec<Message> = entry.messages.into_iter()
                    .map(|m| Message {
                        role: m.role,
                        content: m.content,
                        name: m.name,
                        tool_calls: None,
                        tool_call_id: m.tool_call_id,
                    })
                    .collect();
                Conversation {
                    messages,
                    metadata: None,
                }
            })
            .collect();

        Ok(Self {
            conversations,
            metadata: HashMap::new(),
        })
    }

    pub fn to_sharegpt_format(&self) -> Result<String, String> {
        let entries: Vec<ShareGptEntry> = self.conversations.iter()
            .map(|conv| {
                let conversations: Vec<ShareGptMessage> = conv.messages.iter()
                    .map(|m| ShareGptMessage {
                        from: m.role.clone(),
                        value: m.content.clone(),
                    })
                    .collect();
                ShareGptEntry { conversations }
            })
            .collect();

        serde_json::to_string_pretty(&entries)
            .map_err(|e| format!("Failed to serialize: {}", e))
    }

    pub fn to_openai_format(&self) -> Result<String, String> {
        #[derive(Serialize)]
        struct OpenAiEntry {
            messages: Vec<OpenAiMessageOut>,
        }

        #[derive(Serialize)]
        struct OpenAiMessageOut {
            role: String,
            content: String,
        }

        let entries: Vec<OpenAiEntry> = self.conversations.iter()
            .map(|conv| {
                let messages: Vec<OpenAiMessageOut> = conv.messages.iter()
                    .map(|m| OpenAiMessageOut {
                        role: m.role.clone(),
                        content: m.content.clone(),
                    })
                    .collect();
                OpenAiEntry { messages }
            })
            .collect();

        serde_json::to_string_pretty(&entries)
            .map_err(|e| format!("Failed to serialize: {}", e))
    }

    pub fn len(&self) -> usize {
        self.conversations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.conversations.is_empty()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ShareGptEntry {
    conversations: Vec<ShareGptMessage>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ShareGptMessage {
    from: String,
    value: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llama3_template() {
        let template = ChatTemplate::llama3();
        let mut conv = Conversation::new();
        conv.system("You are a helpful assistant.");
        conv.user("Hello!");
        conv.assistant("Hi there! How can I help?");

        let result = template.apply(&conv, true).unwrap();
        assert!(result.contains("<|start_header_id|>system<|end_header_id|>"));
        assert!(result.contains("<|start_header_id|>user<|end_header_id|>"));
        assert!(result.contains("<|start_header_id|>assistant<|end_header_id|>"));
        assert!(result.contains("Hello!"));
        assert!(result.contains("Hi there!"));
    }

    #[test]
    fn test_chatml_template() {
        let template = ChatTemplate::chatml();
        let mut conv = Conversation::new();
        conv.user("What is AI?");

        let result = template.apply(&conv, true).unwrap();
        assert!(result.contains("<|im_start|>user"));
        assert!(result.contains("What is AI?"));
        assert!(result.contains("<|im_end|>"));
    }

    #[test]
    fn test_sharegpt_parsing() {
        let json = r#"[
            {"conversations": [
                {"from": "human", "value": "Hello"},
                {"from": "gpt", "value": "Hi!"}
            ]}
        ]"#;

        let dataset = ConversationDataset::from_sharegpt_format(json).unwrap();
        assert_eq!(dataset.len(), 1);
        assert_eq!(dataset.conversations[0].messages[0].role, "human");
        assert_eq!(dataset.conversations[0].messages[1].role, "gpt");
    }

    #[test]
    fn test_openai_parsing() {
        let json = r#"[
            {"messages": [
                {"role": "system", "content": "You are helpful."},
                {"role": "user", "content": "Hi"},
                {"role": "assistant", "content": "Hello!"}
            ]}
        ]"#;

        let dataset = ConversationDataset::from_openai_format(json).unwrap();
        assert_eq!(dataset.len(), 1);
        assert_eq!(dataset.conversations[0].messages.len(), 3);
    }
}
