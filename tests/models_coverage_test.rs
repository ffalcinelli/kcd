use kcd::models::*;
use std::collections::HashMap;

#[test]
fn test_models_resource_trait() {
    let mut realm = RealmRepresentation {
        realm: "test".to_string(),
        enabled: Some(true),
        display_name: None,
        extra: HashMap::new(),
    };
    assert_eq!(realm.get_identity(), Some("test".to_string()));
    assert_eq!(realm.get_name(), "test".to_string());
    assert!(!realm.has_id());
    realm.clear_metadata();
    assert_eq!(realm.realm, "test");
    assert_eq!(RealmRepresentation::dir_name(), "realms");
    assert_eq!(RealmRepresentation::api_path(), "realms");

    let mut idp = IdentityProviderRepresentation {
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
    assert!(idp.has_id());
    idp.clear_metadata();
    assert!(idp.internal_id.is_none());
    assert_eq!(IdentityProviderRepresentation::dir_name(), "identity-providers");
    assert_eq!(IdentityProviderRepresentation::label(), "identity providers");
    assert_eq!(IdentityProviderRepresentation::secret_prefix(), "idp");

    let mut client = ClientRepresentation {
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
    assert_eq!(client.get_identity(), Some("cid".to_string()));
    assert_eq!(client.get_name(), "cid".to_string());
    assert!(client.has_id());
    client.clear_metadata();
    assert!(client.id.is_none());
    assert_eq!(ClientRepresentation::dir_name(), "clients");
    assert_eq!(ClientRepresentation::label(), "clients");
    assert_eq!(ClientRepresentation::secret_prefix(), "client");

    let mut role = RoleRepresentation {
        id: Some("id3".to_string()),
        name: "rname".to_string(),
        description: None,
        container_id: Some("c1".to_string()),
        composite: false,
        client_role: false,
        extra: HashMap::new(),
    };
    assert_eq!(role.get_identity(), Some("rname".to_string()));
    assert_eq!(role.get_name(), "rname".to_string());
    assert!(role.has_id());
    role.clear_metadata();
    assert!(role.id.is_none());
    assert!(role.container_id.is_none());
    assert_eq!(RoleRepresentation::dir_name(), "roles");
    assert_eq!(RoleRepresentation::label(), "roles");
    assert_eq!(RoleRepresentation::secret_prefix(), "role");

    let mut group = GroupRepresentation {
        id: Some("id4".to_string()),
        name: Some("gname".to_string()),
        path: Some("/gname".to_string()),
        sub_groups: None,
        extra: HashMap::new(),
    };
    assert_eq!(group.get_identity(), Some("/gname".to_string()));
    assert_eq!(group.get_name(), "gname".to_string());
    assert!(group.has_id());
    group.clear_metadata();
    assert!(group.id.is_none());
    assert_eq!(GroupRepresentation::dir_name(), "groups");
    assert_eq!(GroupRepresentation::label(), "groups");
    assert_eq!(GroupRepresentation::secret_prefix(), "group");

    let mut user = UserRepresentation {
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
    assert_eq!(user.get_identity(), Some("uname".to_string()));
    assert_eq!(user.get_name(), "uname".to_string());
    assert!(user.has_id());
    user.clear_metadata();
    assert!(user.id.is_none());
    assert_eq!(UserRepresentation::dir_name(), "users");
    assert_eq!(UserRepresentation::label(), "users");
    assert_eq!(UserRepresentation::secret_prefix(), "user");

    let mut scope = ClientScopeRepresentation {
        id: Some("id6".to_string()),
        name: Some("sname".to_string()),
        description: None,
        protocol: Some("openid-connect".to_string()),
        attributes: None,
        extra: HashMap::new(),
    };
    assert_eq!(scope.get_identity(), Some("sname".to_string()));
    assert_eq!(scope.get_name(), "sname".to_string());
    assert!(scope.has_id());
    scope.clear_metadata();
    assert!(scope.id.is_none());
    assert_eq!(ClientScopeRepresentation::dir_name(), "client-scopes");
    assert_eq!(ClientScopeRepresentation::label(), "client scopes");
    assert_eq!(ClientScopeRepresentation::secret_prefix(), "client_scope");

    let mut flow = AuthenticationFlowRepresentation {
        id: Some("id7".to_string()),
        alias: Some("falias".to_string()),
        description: None,
        provider_id: Some("p1".to_string()),
        top_level: Some(true),
        built_in: Some(false),
        authentication_executions: None,
        extra: HashMap::new(),
    };
    assert_eq!(flow.get_identity(), Some("falias".to_string()));
    assert_eq!(flow.get_name(), "falias".to_string());
    assert!(flow.has_id());
    flow.clear_metadata();
    assert!(flow.id.is_none());
    assert_eq!(AuthenticationFlowRepresentation::dir_name(), "authentication-flows");
    assert_eq!(AuthenticationFlowRepresentation::label(), "authentication flows");
    assert_eq!(AuthenticationFlowRepresentation::secret_prefix(), "flow");

    let mut action = RequiredActionProviderRepresentation {
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
    assert!(!action.has_id()); // RequiredActionProviderRepresentation doesn't have has_id impl, so it uses default (false)
    action.clear_metadata();
    assert!(action.alias.is_some());
    assert_eq!(RequiredActionProviderRepresentation::dir_name(), "required-actions");
    assert_eq!(RequiredActionProviderRepresentation::label(), "required actions");
    assert_eq!(RequiredActionProviderRepresentation::secret_prefix(), "action");

    let mut comp = ComponentRepresentation {
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
    assert!(comp.has_id());
    comp.clear_metadata();
    assert!(comp.id.is_none());
    assert_eq!(ComponentRepresentation::dir_name(), "components");
    assert_eq!(ComponentRepresentation::label(), "components");
    assert_eq!(ComponentRepresentation::secret_prefix(), "component");
}
