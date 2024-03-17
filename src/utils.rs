use uuid::Uuid;
use ::windows::core::GUID;

/// windows GUID to Uuid
pub(crate) fn to_uuid(guid: &GUID) -> Uuid {
    Uuid::from_u128(guid.to_u128())
}