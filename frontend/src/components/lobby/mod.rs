// lobby/mod.rs - Main lobby component
pub mod lobby_join;
pub mod lobby_management;
pub mod settings;

// Re-export shared components
pub use lobby_management::{GameInstructions, LobbyManagementComponent, StatusMessage};
pub use lobby_join::LobbyJoinComponent;
