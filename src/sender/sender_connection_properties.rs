use crate::connection_properties::ConnectionProperties;

pub struct SenderConnectionProperties {
    pub static_properties: ConnectionProperties,
    pub window_position: u16,
}

impl SenderConnectionProperties {
    pub fn new(props: ConnectionProperties) -> Self {
        Self {
            static_properties: props,
            window_position: 0,
        }
    }
}