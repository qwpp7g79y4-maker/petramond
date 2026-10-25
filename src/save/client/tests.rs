use super::*;

#[test]
fn a_signed_in_account_outranks_the_configured_player_name() {
    let configured = ClientSettings {
        player_name: Some("rachel".to_string()),
        ..ClientSettings::default()
    };
    assert_eq!(
        player_name_from(&configured, Some(" Explorer ".to_string())),
        "Explorer",
        "the account username wins, trimmed"
    );
    assert_eq!(
        player_name_from(&configured, None),
        "rachel",
        "signed out, the configured name still answers"
    );
    assert_eq!(
        player_name_from(&configured, Some("   ".to_string())),
        "rachel",
        "a blank account name falls through instead of blanking the identity"
    );
}
