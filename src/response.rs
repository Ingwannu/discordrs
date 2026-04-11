use serde_json::Value;

use crate::builders::{ComponentsV2Message, ModalBuilder};
use crate::constants::MESSAGE_FLAG_IS_COMPONENTS_V2;
use crate::error::DiscordError;
use crate::helpers::{
    INTERACTION_RESPONSE_CHANNEL_MESSAGE, INTERACTION_RESPONSE_DEFERRED_CHANNEL_MESSAGE,
    INTERACTION_RESPONSE_MODAL, INTERACTION_RESPONSE_UPDATE_MESSAGE,
};
use crate::model::{CreateMessage, InteractionCallbackResponse};

#[derive(Clone, Debug, Default)]
pub struct MessageBuilder {
    inner: CreateMessage,
}

impl MessageBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.inner.content = Some(content.into());
        self
    }

    pub fn components(mut self, components: Vec<Value>) -> Self {
        self.inner.components = Some(components);
        self
    }

    pub fn components_v2(mut self, message: ComponentsV2Message) -> Self {
        self.inner.components = Some(message.build());
        self.inner.flags = Some(self.inner.flags.unwrap_or(0) | MESSAGE_FLAG_IS_COMPONENTS_V2);
        self
    }

    pub fn flags(mut self, flags: u64) -> Self {
        self.inner.flags = Some(flags);
        self
    }

    pub fn ephemeral(mut self, ephemeral: bool) -> Self {
        if ephemeral {
            self.inner.flags = Some(self.inner.flags.unwrap_or(0) | (1 << 6));
        }
        self
    }

    pub fn build(self) -> CreateMessage {
        self.inner
    }
}

#[derive(Clone, Debug)]
pub struct InteractionResponseBuilder {
    inner: InteractionCallbackResponse,
}

impl InteractionResponseBuilder {
    pub fn channel_message(message: MessageBuilder) -> Result<Self, DiscordError> {
        Ok(Self {
            inner: InteractionCallbackResponse {
                kind: INTERACTION_RESPONSE_CHANNEL_MESSAGE,
                data: Some(serde_json::to_value(message.build())?),
            },
        })
    }

    pub fn deferred_channel_message(ephemeral: bool) -> Self {
        let mut flags = 0_u64;
        if ephemeral {
            flags |= 1 << 6;
        }

        Self {
            inner: InteractionCallbackResponse {
                kind: INTERACTION_RESPONSE_DEFERRED_CHANNEL_MESSAGE,
                data: Some(serde_json::json!({ "flags": flags })),
            },
        }
    }

    pub fn update_message(message: MessageBuilder) -> Result<Self, DiscordError> {
        Ok(Self {
            inner: InteractionCallbackResponse {
                kind: INTERACTION_RESPONSE_UPDATE_MESSAGE,
                data: Some(serde_json::to_value(message.build())?),
            },
        })
    }

    pub fn modal(modal: ModalBuilder) -> Result<Self, DiscordError> {
        Ok(Self {
            inner: InteractionCallbackResponse {
                kind: INTERACTION_RESPONSE_MODAL,
                data: Some(serde_json::to_value(modal.build())?),
            },
        })
    }

    pub fn build(self) -> InteractionCallbackResponse {
        self.inner
    }
}

#[cfg(test)]
mod tests {
    use crate::builders::ComponentsV2Message;

    use super::{InteractionResponseBuilder, MessageBuilder};

    #[test]
    fn message_builder_sets_ephemeral_components_v2_flags() {
        let message = MessageBuilder::new()
            .components_v2(ComponentsV2Message::new())
            .ephemeral(true)
            .build();

        assert_eq!(message.flags, Some((1 << 15) | (1 << 6)));
    }

    #[test]
    fn interaction_response_builder_wraps_message_payload() {
        let response =
            InteractionResponseBuilder::channel_message(MessageBuilder::new().content("hello"))
                .unwrap()
                .build();

        assert_eq!(response.kind, 4);
        assert_eq!(response.data.unwrap()["content"], "hello");
    }
}
