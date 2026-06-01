use std::path::Path;

/// Olá, patinho! Este é o `build.rs`, um script especial que roda *antes* do compilador de Rust.
/// Ele funciona como um assistente de preparação. A missão dele é ler as configurações do nosso
/// arquivo `.env` (que fica na raiz do projeto) e transformá-las em variáveis que o compilador
/// do Rust possa embutir direto dentro do binário final do nosso agente Mercator.
///
/// Se alguma variável importante não estiver no `.env`, nós definimos ela como vazia para que
/// o projeto compile sem dar erro, mas mostre um aviso bem bonito quando rodar!
fn main() {
    // 1. Patinho, primeiro precisamos descobrir onde fica a raiz do nosso projeto (workspace).
    // O Cargo nos dá o diretório deste agente (Cargo.toml). Subimos duas pastas para chegar na raiz!
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("Patinho, não consegui encontrar a pasta raiz do workspace!");

    // 2. Agora que sabemos onde é a raiz, nós apontamos direto para o arquivo `.env`.
    let dotenv_path = workspace_root.join(".env");

    println!("cargo:warning=Patinho, a pasta raiz resolvida eh: {}", workspace_root.display());
    println!("cargo:warning=Patinho, o arquivo .env existe? {}", dotenv_path.exists());

    // 3. Avisamos o Cargo (o compilador): "Ei, se o arquivo `.env` mudar, por favor, rode esse script
    // de novo para atualizarmos as variáveis embutidas no binário!"
    println!("cargo:rerun-if-changed={}", dotenv_path.display());

    let mut loaded_vars = std::collections::HashSet::new();

    // 4. Se o arquivo `.env` realmente existir lá...
    if dotenv_path.exists() {
        // Lemos todas as linhas dele usando o ajudante `dotenvy` e iteramos sobre cada par chave-valor.
        for (key, value) in dotenvy::from_path_iter(&dotenv_path).expect("Patinho, falhei ao ler o arquivo .env").flatten() {
            println!("cargo:warning=Patinho, lido do .env: {key}={value}");
            // Apenas repassamos para o binário as variáveis de compilação que nos interessam.
            if is_build_var(&key) {
                println!("cargo:warning=Patinho, registrando variavel de build: {key}={value}");
                // Ao imprimir essa linha mágica, o Cargo entende que deve injetar essa variável
                // no ambiente de compilação. Depois, no código Rust, usamos `env!("NOME_DA_VAR")` para ler!
                println!("cargo:rustc-env={key}={value}");
                loaded_vars.insert(key.clone());
            }
        }
    }

    // 5. E se alguma das variáveis que precisamos não estava no arquivo `.env`?
    // Também checamos se ela foi passada no ambiente real do sistema operacional (OS env).
    for var in BUILD_VARS {
        if !loaded_vars.contains(*var) {
            // Se não foi carregada do .env, confere se está no ambiente do SO
            if let Ok(value) = std::env::var(var) {
                println!("cargo:rustc-env={var}={value}");
            } else {
                // Se não estiver em lugar nenhum, põe vazio para não quebrar a compilação!
                println!("cargo:rustc-env={var}=");
            }
        }
    }
}

/// Patinho, estas são as quatro variáveis ultra-importantes que queremos embutir no binário:
/// 1. URL do servidor Mercator.
/// 2. Chave de handshake (usada para o primeiro contato do device).
/// 3. ID da chave pública para criptografar a localização.
/// 4. A própria chave pública de localização (em base64).
const BUILD_VARS: &[&str] = &[
    "MERCATOR_SERVER_URL",
    "MERCATOR_HANDSHAKE_KEY",
    "MERCATOR_LOCATION_PUBLIC_KEY_ID",
    "MERCATOR_LOCATION_PUBLIC_KEY",
    "MERCATOR_LOCATION_PUBLIC_KEY_BASE64",
];

/// Uma funçãozinha simples para conferir se uma chave do `.env` é uma das quatro variáveis
/// que definimos na nossa lista `BUILD_VARS`.
fn is_build_var(key: &str) -> bool {
    BUILD_VARS.contains(&key)
}

