use app::models::*;
use std::collections::HashMap;

#[test]
fn test_models_resource_trait() {
    let realm = RealmRepresentation {
        realm: "test".to_string(),
        enabled: Some(true),
        display_name: None,
        extra: HashMap::new(),
    };
    assert_eq!(realm.get_identity(), Some("test".to_string()));
    assert_eq!(realm.get_name(), "test".to_string());

    let idp = IdentityProviderRepresentation {
        internal_id: Some("id1".to_string()),
        alias: Some("alias1".to_string()),
        display_name: None,
        enabled: Some(true),
        provider_id: Some("google".to_string()),
        config: None,
        update_profile_first_login_mode: None,
        trust_email: None,
        store_token: None,
        add_read_token_role_on_create: None,
        authenticate_by_default: None,
        link_only: None,
        first_broker_login_flow_alias: None,
        post_broker_login_flow_alias: None,
        extra: HashMap::new(),
    };
    assert_eq!(idp.get_identity(), Some("alias1".to_string()));
    assert_eq!(idp.get_name(), "alias1".to_string());

    let client = ClientRepresentation {
        id: Some("id2".to_string()),
        client_id: Some("cid".to_string()),
        name: Some("cname".to_string()),
        description: None,
        enabled: Some(true),
        protocol: None,
        redirect_uris: None,
        web_origins: None,
        public_client: None,
        bearer_only: None,
        service_accounts_enabled: None,
        extra: HashMap::new(),
    };
    assert_eq!(client.get_identity(), Some("id2".to_string()));
    assert_eq!(client.get_name(), "cid".to_string());

    let role = RoleRepresentation {
        id: Some("id3".to_string()),
        name: "rname".to_string(),
        description: None,
        container_id: None,
        composite: false,
        client_role: false,
        extra: HashMap::new(),
    };
    assert_eq!(role.get_identity(), Some("id3".to_string()));
    assert_eq!(role.get_name(), "rname".to_string());

    let group = GroupRepresentation {
        id: Some("id4".to_string()),
        name: Some("gname".to_string()),
        path: Some("/gname".to_string()),
        sub_groups: None,
        extra: HashMap::new(),
    };
    assert_eq!(group.get_identity(), Some("id4".to_string()));
    assert_eq!(group.get_name(), "gname".to_string());

    let user = UserRepresentation {
        id: Some("id5".to_string()),
        username: Some("uname".to_string()),
        enabled: Some(true),
        email: None,
        first_name: None,
        last_name: None,
        email_verified: None,
        credentials: None,
        extra: HashMap::new(),
    };
    assert_eq!(user.get_identity(), Some("id5".to_string()));
    assert_eq!(user.get_name(), "uname".to_string());

    let scope = ClientScopeRepresentation {
        id: Some("id6".to_string()),
        name: Some("sname".to_string()),
        description: None,
        protocol: Some("openid-connect".to_string()),
        attributes: None,
        extra: HashMap::new(),
    };
    assert_eq!(scope.get_identity(), Some("id6".to_string()));
    assert_eq!(scope.get_name(), "sname".to_string());

    let flow = AuthenticationFlowRepresentation {
        id: Some("id7".to_string()),
        alias: Some("falias".to_string()),
        description: None,
        provider_id: Some("p1".to_string()),
        top_level: Some(true),
        built_in: Some(false),
        authentication_executions: None,
        extra: HashMap::new(),
    };
    assert_eq!(flow.get_identity(), Some("id7".to_string()));
    assert_eq!(flow.get_name(), "falias".to_string());

    let action = RequiredActionProviderRepresentation {
        alias: Some("aalias".to_string()),
        name: Some("aname".to_string()),
        provider_id: Some("p2".to_string()),
        enabled: Some(true),
        default_action: Some(false),
        priority: Some(10),
        config: None,
        extra: HashMap::new(),
    };
    assert_eq!(action.get_identity(), Some("aalias".to_string()));
    assert_eq!(action.get_name(), "aalias".to_string());

    let comp = ComponentRepresentation {
        id: Some("id8".to_string()),
        name: Some("cname".to_string()),
        provider_id: Some("p3".to_string()),
        provider_type: Some("t1".to_string()),
        parent_id: None,
        sub_type: None,
        config: None,
        extra: HashMap::new(),
    };
    assert_eq!(comp.get_identity(), Some("id8".to_string()));
    assert_eq!(comp.get_name(), "cname".to_string());
}
