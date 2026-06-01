/// Olá, patinho! Esta função especial tenta coletar o nome do usuário ativo (logado)
/// na máquina a partir das variáveis de ambiente padrão do sistema operacional.
///
/// **Lógica simples:**
/// 1. Nós olhamos a variável `USERNAME` (padrão no Windows) ou `USER` (padrão em sistemas Unix como Linux/macOS).
/// 2. Se a máquina estiver conectada em um Domínio de Rede Corporativo do Windows (Active Directory),
///    nós também buscamos o domínio (`USERDOMAIN`) e retornamos no formato completo: `DOMINIO\usuario`!
///    Se não houver domínio, retornamos apenas o nome do usuário.
#[allow(dead_code)]
pub fn collect_current_user() -> Option<String> {
    let username = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())?;

    if let Ok(domain) = std::env::var("USERDOMAIN") {
        let domain = domain.trim();
        if !domain.is_empty() {
            return Some(format!("{domain}\\{username}"));
        }
    }

    Some(username)
}

