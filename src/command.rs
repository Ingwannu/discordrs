use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::model::{
    ApplicationCommand, ApplicationCommandHandlerType, ApplicationCommandOption,
    ApplicationCommandOptionChoice, ApplicationIntegrationType, InteractionContextType,
    PermissionsBitField,
};

pub mod command_type {
    pub const CHAT_INPUT: u8 = 1;
    pub const USER: u8 = 2;
    pub const MESSAGE: u8 = 3;
    pub const PRIMARY_ENTRY_POINT: u8 = 4;
}

pub mod option_type {
    pub const SUB_COMMAND: u8 = 1;
    pub const SUB_COMMAND_GROUP: u8 = 2;
    pub const STRING: u8 = 3;
    pub const INTEGER: u8 = 4;
    pub const BOOLEAN: u8 = 5;
    pub const USER: u8 = 6;
    pub const CHANNEL: u8 = 7;
    pub const ROLE: u8 = 8;
    pub const MENTIONABLE: u8 = 9;
    pub const NUMBER: u8 = 10;
    pub const ATTACHMENT: u8 = 11;
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CommandDefinition {
    #[serde(rename = "type")]
    pub kind: u8,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_localizations: Option<HashMap<String, String>>,
    #[serde(default)]
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_localizations: Option<HashMap<String, String>>,
    #[serde(default)]
    pub options: Vec<ApplicationCommandOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_member_permissions: Option<PermissionsBitField>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dm_permission: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration_types: Option<Vec<ApplicationIntegrationType>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contexts: Option<Vec<InteractionContextType>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handler: Option<ApplicationCommandHandlerType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nsfw: Option<bool>,
}

impl From<CommandDefinition> for ApplicationCommand {
    fn from(value: CommandDefinition) -> Self {
        ApplicationCommand {
            id: None,
            application_id: None,
            guild_id: None,
            kind: value.kind,
            name: value.name,
            name_localizations: value.name_localizations,
            description: value.description,
            description_localizations: value.description_localizations,
            options: value.options,
            default_member_permissions: value.default_member_permissions,
            dm_permission: value.dm_permission,
            integration_types: value.integration_types,
            contexts: value.contexts,
            handler: value.handler,
            version: None,
            nsfw: value.nsfw,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CommandOptionBuilder {
    inner: ApplicationCommandOption,
}

impl CommandOptionBuilder {
    pub fn new(kind: u8, name: &str, description: &str) -> Self {
        Self {
            inner: ApplicationCommandOption {
                kind,
                name: name.to_string(),
                description: description.to_string(),
                ..ApplicationCommandOption::default()
            },
        }
    }

    pub fn required(mut self, required: bool) -> Self {
        self.inner.required = Some(required);
        self
    }

    pub fn autocomplete(mut self, enabled: bool) -> Self {
        self.inner.autocomplete = Some(enabled);
        self
    }

    pub fn choice(mut self, name: &str, value: impl Serialize) -> Self {
        self.inner.choices.push(ApplicationCommandOptionChoice {
            name: name.to_string(),
            value: serde_json::to_value(value).expect("failed to serialize command option choice"),
        });
        self
    }

    pub fn min_value(mut self, value: f64) -> Self {
        self.inner.min_value = Some(value);
        self
    }

    pub fn max_value(mut self, value: f64) -> Self {
        self.inner.max_value = Some(value);
        self
    }

    pub fn min_length(mut self, value: u16) -> Self {
        self.inner.min_length = Some(value);
        self
    }

    pub fn max_length(mut self, value: u16) -> Self {
        self.inner.max_length = Some(value);
        self
    }

    pub fn option(mut self, option: CommandOptionBuilder) -> Self {
        self.inner.options.push(option.build());
        self
    }

    pub fn subcommand(name: &str, description: &str) -> Self {
        Self::new(option_type::SUB_COMMAND, name, description)
    }

    pub fn subcommand_group(name: &str, description: &str) -> Self {
        Self::new(option_type::SUB_COMMAND_GROUP, name, description)
    }

    pub fn string(name: &str, description: &str) -> Self {
        Self::new(option_type::STRING, name, description)
    }

    pub fn integer(name: &str, description: &str) -> Self {
        Self::new(option_type::INTEGER, name, description)
    }

    pub fn boolean(name: &str, description: &str) -> Self {
        Self::new(option_type::BOOLEAN, name, description)
    }

    pub fn user(name: &str, description: &str) -> Self {
        Self::new(option_type::USER, name, description)
    }

    pub fn channel(name: &str, description: &str) -> Self {
        Self::new(option_type::CHANNEL, name, description)
    }

    pub fn role(name: &str, description: &str) -> Self {
        Self::new(option_type::ROLE, name, description)
    }

    pub fn mentionable(name: &str, description: &str) -> Self {
        Self::new(option_type::MENTIONABLE, name, description)
    }

    pub fn number(name: &str, description: &str) -> Self {
        Self::new(option_type::NUMBER, name, description)
    }

    pub fn attachment(name: &str, description: &str) -> Self {
        Self::new(option_type::ATTACHMENT, name, description)
    }

    pub fn build(self) -> ApplicationCommandOption {
        self.inner
    }
}

#[derive(Clone, Debug, Default)]
pub struct SlashCommandBuilder {
    inner: CommandDefinition,
}

impl SlashCommandBuilder {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            inner: CommandDefinition {
                kind: command_type::CHAT_INPUT,
                name: name.to_string(),
                description: description.to_string(),
                ..CommandDefinition::default()
            },
        }
    }

    pub fn option(mut self, option: CommandOptionBuilder) -> Self {
        self.inner.options.push(option.build());
        self
    }

    pub fn string_option(self, name: &str, description: &str, required: bool) -> Self {
        self.option(CommandOptionBuilder::string(name, description).required(required))
    }

    pub fn integer_option(self, name: &str, description: &str, required: bool) -> Self {
        self.option(CommandOptionBuilder::integer(name, description).required(required))
    }

    pub fn boolean_option(self, name: &str, description: &str, required: bool) -> Self {
        self.option(CommandOptionBuilder::boolean(name, description).required(required))
    }

    pub fn user_option(self, name: &str, description: &str, required: bool) -> Self {
        self.option(CommandOptionBuilder::user(name, description).required(required))
    }

    pub fn subcommand(self, option: CommandOptionBuilder) -> Self {
        self.option(option)
    }

    pub fn default_member_permissions(mut self, permissions: PermissionsBitField) -> Self {
        self.inner.default_member_permissions = Some(permissions);
        self
    }

    pub fn dm_permission(mut self, enabled: bool) -> Self {
        self.inner.dm_permission = Some(enabled);
        self
    }

    pub fn nsfw(mut self, enabled: bool) -> Self {
        self.inner.nsfw = Some(enabled);
        self
    }

    pub fn integration_types<I>(mut self, integration_types: I) -> Self
    where
        I: IntoIterator<Item = ApplicationIntegrationType>,
    {
        self.inner.integration_types = Some(integration_types.into_iter().collect());
        self
    }

    pub fn contexts<I>(mut self, contexts: I) -> Self
    where
        I: IntoIterator<Item = InteractionContextType>,
    {
        self.inner.contexts = Some(contexts.into_iter().collect());
        self
    }

    pub fn name_localization(mut self, locale: &str, name: &str) -> Self {
        self.inner
            .name_localizations
            .get_or_insert_with(HashMap::new)
            .insert(locale.to_string(), name.to_string());
        self
    }

    pub fn description_localization(mut self, locale: &str, description: &str) -> Self {
        self.inner
            .description_localizations
            .get_or_insert_with(HashMap::new)
            .insert(locale.to_string(), description.to_string());
        self
    }

    pub fn handler(mut self, handler: ApplicationCommandHandlerType) -> Self {
        self.inner.handler = Some(handler);
        self
    }

    pub fn build(self) -> CommandDefinition {
        self.inner
    }
}

#[derive(Clone, Debug, Default)]
pub struct PrimaryEntryPointCommandBuilder {
    inner: CommandDefinition,
}

impl PrimaryEntryPointCommandBuilder {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            inner: CommandDefinition {
                kind: command_type::PRIMARY_ENTRY_POINT,
                name: name.to_string(),
                description: description.to_string(),
                ..CommandDefinition::default()
            },
        }
    }

    pub fn integration_types<I>(mut self, integration_types: I) -> Self
    where
        I: IntoIterator<Item = ApplicationIntegrationType>,
    {
        self.inner.integration_types = Some(integration_types.into_iter().collect());
        self
    }

    pub fn contexts<I>(mut self, contexts: I) -> Self
    where
        I: IntoIterator<Item = InteractionContextType>,
    {
        self.inner.contexts = Some(contexts.into_iter().collect());
        self
    }

    pub fn name_localization(mut self, locale: &str, name: &str) -> Self {
        self.inner
            .name_localizations
            .get_or_insert_with(HashMap::new)
            .insert(locale.to_string(), name.to_string());
        self
    }

    pub fn description_localization(mut self, locale: &str, description: &str) -> Self {
        self.inner
            .description_localizations
            .get_or_insert_with(HashMap::new)
            .insert(locale.to_string(), description.to_string());
        self
    }

    pub fn handler(mut self, handler: ApplicationCommandHandlerType) -> Self {
        self.inner.handler = Some(handler);
        self
    }

    pub fn build(self) -> CommandDefinition {
        self.inner
    }
}

#[derive(Clone, Debug, Default)]
pub struct UserCommandBuilder {
    inner: CommandDefinition,
}

impl UserCommandBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            inner: CommandDefinition {
                kind: command_type::USER,
                name: name.to_string(),
                ..CommandDefinition::default()
            },
        }
    }

    pub fn default_member_permissions(mut self, permissions: PermissionsBitField) -> Self {
        self.inner.default_member_permissions = Some(permissions);
        self
    }

    pub fn dm_permission(mut self, enabled: bool) -> Self {
        self.inner.dm_permission = Some(enabled);
        self
    }

    pub fn build(self) -> CommandDefinition {
        self.inner
    }
}

#[derive(Clone, Debug, Default)]
pub struct MessageCommandBuilder {
    inner: CommandDefinition,
}

impl MessageCommandBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            inner: CommandDefinition {
                kind: command_type::MESSAGE,
                name: name.to_string(),
                ..CommandDefinition::default()
            },
        }
    }

    pub fn default_member_permissions(mut self, permissions: PermissionsBitField) -> Self {
        self.inner.default_member_permissions = Some(permissions);
        self
    }

    pub fn dm_permission(mut self, enabled: bool) -> Self {
        self.inner.dm_permission = Some(enabled);
        self
    }

    pub fn build(self) -> CommandDefinition {
        self.inner
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        command_type, option_type, CommandOptionBuilder, MessageCommandBuilder,
        PrimaryEntryPointCommandBuilder, SlashCommandBuilder, UserCommandBuilder,
    };
    use crate::model::{
        ApplicationCommand, ApplicationCommandHandlerType, ApplicationIntegrationType,
        InteractionContextType, PermissionsBitField,
    };

    #[test]
    fn slash_command_builder_serializes_nested_options() {
        let command = SlashCommandBuilder::new("hello", "Say hello")
            .option(
                CommandOptionBuilder::new(option_type::STRING, "target", "Target user")
                    .required(true)
                    .choice("World", "world"),
            )
            .build();

        let value = serde_json::to_value(command).unwrap();
        assert_eq!(value["type"], json!(command_type::CHAT_INPUT));
        assert_eq!(value["options"][0]["name"], json!("target"));
        assert_eq!(value["options"][0]["choices"][0]["value"], json!("world"));
    }

    #[test]
    fn slash_command_builder_exposes_common_option_shortcuts() {
        let command = SlashCommandBuilder::new("moderate", "Moderation command")
            .string_option("reason", "Reason", true)
            .boolean_option("silent", "Whether the reply should be hidden", false)
            .build();

        let value = serde_json::to_value(command).unwrap();
        assert_eq!(value["options"][0]["type"], json!(option_type::STRING));
        assert_eq!(value["options"][1]["type"], json!(option_type::BOOLEAN));
    }

    #[test]
    fn command_option_builder_shortcuts_cover_supported_types() {
        let cases = [
            (
                CommandOptionBuilder::subcommand("sub", "desc").build(),
                option_type::SUB_COMMAND,
            ),
            (
                CommandOptionBuilder::subcommand_group("group", "desc").build(),
                option_type::SUB_COMMAND_GROUP,
            ),
            (
                CommandOptionBuilder::string("string", "desc").build(),
                option_type::STRING,
            ),
            (
                CommandOptionBuilder::integer("integer", "desc").build(),
                option_type::INTEGER,
            ),
            (
                CommandOptionBuilder::boolean("boolean", "desc").build(),
                option_type::BOOLEAN,
            ),
            (
                CommandOptionBuilder::user("user", "desc").build(),
                option_type::USER,
            ),
            (
                CommandOptionBuilder::channel("channel", "desc").build(),
                option_type::CHANNEL,
            ),
            (
                CommandOptionBuilder::role("role", "desc").build(),
                option_type::ROLE,
            ),
            (
                CommandOptionBuilder::mentionable("mentionable", "desc").build(),
                option_type::MENTIONABLE,
            ),
            (
                CommandOptionBuilder::number("number", "desc").build(),
                option_type::NUMBER,
            ),
            (
                CommandOptionBuilder::attachment("attachment", "desc").build(),
                option_type::ATTACHMENT,
            ),
        ];

        for (option, expected_kind) in cases {
            assert_eq!(option.kind, expected_kind);
        }
    }

    #[test]
    fn command_option_builder_serializes_nested_constraints_and_choices() {
        let option = CommandOptionBuilder::subcommand_group("admin", "Admin tools")
            .option(
                CommandOptionBuilder::subcommand("ban", "Ban a member")
                    .option(CommandOptionBuilder::user("target", "Member").required(true))
                    .option(
                        CommandOptionBuilder::integer("days", "Delete days")
                            .autocomplete(true)
                            .min_value(1.0)
                            .max_value(7.0),
                    )
                    .option(
                        CommandOptionBuilder::string("reason", "Reason")
                            .min_length(3)
                            .max_length(120),
                    )
                    .option(CommandOptionBuilder::number("ratio", "Ratio").choice("Half", 0.5_f64))
                    .option(CommandOptionBuilder::attachment("proof", "Proof")),
            )
            .build();

        let value = serde_json::to_value(option).unwrap();
        let nested = &value["options"][0]["options"];

        assert_eq!(value["type"], json!(option_type::SUB_COMMAND_GROUP));
        assert_eq!(value["options"][0]["type"], json!(option_type::SUB_COMMAND));
        assert_eq!(nested[0]["required"], json!(true));
        assert_eq!(nested[1]["autocomplete"], json!(true));
        assert_eq!(nested[1]["min_value"], json!(1.0));
        assert_eq!(nested[1]["max_value"], json!(7.0));
        assert_eq!(nested[2]["min_length"], json!(3));
        assert_eq!(nested[2]["max_length"], json!(120));
        assert_eq!(nested[3]["choices"][0]["name"], json!("Half"));
        assert_eq!(nested[3]["choices"][0]["value"], json!(0.5));
        assert_eq!(nested[4]["type"], json!(option_type::ATTACHMENT));
    }

    #[test]
    fn command_definition_converts_into_application_command() {
        let permissions = PermissionsBitField(8);
        let command = SlashCommandBuilder::new("ban", "Ban a member")
            .integer_option("days", "Delete days", false)
            .default_member_permissions(permissions)
            .dm_permission(false)
            .nsfw(true)
            .integration_types([
                ApplicationIntegrationType::GUILD_INSTALL,
                ApplicationIntegrationType::USER_INSTALL,
            ])
            .contexts([
                InteractionContextType::GUILD,
                InteractionContextType::BOT_DM,
            ])
            .name_localization("ko", "차단")
            .description_localization("ko", "멤버 차단")
            .handler(ApplicationCommandHandlerType::APP_HANDLER)
            .build();

        let application_command: ApplicationCommand = command.clone().into();

        assert_eq!(application_command.kind, command_type::CHAT_INPUT);
        assert_eq!(application_command.name, "ban");
        assert_eq!(application_command.description, "Ban a member");
        assert_eq!(application_command.options.len(), 1);
        assert_eq!(
            application_command
                .default_member_permissions
                .map(PermissionsBitField::bits),
            Some(8)
        );
        assert_eq!(application_command.dm_permission, Some(false));
        assert_eq!(application_command.nsfw, Some(true));
        assert_eq!(
            application_command.integration_types,
            Some(vec![
                ApplicationIntegrationType::GUILD_INSTALL,
                ApplicationIntegrationType::USER_INSTALL
            ])
        );
        assert_eq!(
            application_command.contexts,
            Some(vec![
                InteractionContextType::GUILD,
                InteractionContextType::BOT_DM
            ])
        );
        assert_eq!(
            application_command
                .name_localizations
                .as_ref()
                .and_then(|localizations| localizations.get("ko"))
                .map(String::as_str),
            Some("차단")
        );
        assert_eq!(
            application_command.handler,
            Some(ApplicationCommandHandlerType::APP_HANDLER)
        );
    }

    #[test]
    fn primary_entry_point_builder_serializes_activity_fields() {
        let command = PrimaryEntryPointCommandBuilder::new("launch", "Launch activity")
            .integration_types([
                ApplicationIntegrationType::GUILD_INSTALL,
                ApplicationIntegrationType::USER_INSTALL,
            ])
            .contexts([
                InteractionContextType::GUILD,
                InteractionContextType::BOT_DM,
            ])
            .handler(ApplicationCommandHandlerType::DISCORD_LAUNCH_ACTIVITY)
            .build();

        let value = serde_json::to_value(command).unwrap();
        assert_eq!(value["type"], json!(command_type::PRIMARY_ENTRY_POINT));
        assert_eq!(value["integration_types"], json!([0, 1]));
        assert_eq!(value["contexts"], json!([0, 1]));
        assert_eq!(value["handler"], json!(2));
    }

    #[test]
    fn user_and_message_command_builders_apply_command_kinds_and_permissions() {
        let user_command = UserCommandBuilder::new("Inspect")
            .default_member_permissions(PermissionsBitField(16))
            .dm_permission(true)
            .build();
        let message_command = MessageCommandBuilder::new("Quote")
            .default_member_permissions(PermissionsBitField(32))
            .dm_permission(false)
            .build();

        let user_value = serde_json::to_value(user_command).unwrap();
        let message_value = serde_json::to_value(message_command).unwrap();

        assert_eq!(user_value["type"], json!(command_type::USER));
        assert_eq!(user_value["default_member_permissions"], json!("16"));
        assert_eq!(user_value["dm_permission"], json!(true));
        assert_eq!(message_value["type"], json!(command_type::MESSAGE));
        assert_eq!(message_value["default_member_permissions"], json!("32"));
        assert_eq!(message_value["dm_permission"], json!(false));
    }
}
