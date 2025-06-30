macro_rules! find_repository {
    ($name:ident, $body:block) => {{
        let $name = Repository::find_repository(&env::current_dir().unwrap().as_path()).unwrap();
        $body
    }};
}

pub(crate) use find_repository;
