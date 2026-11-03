# Segurança

O FlowVoice guarda chaves de API e tem acesso ao microfone e à área de transferência, então falhas de segurança são levadas a sério.

## Como reportar uma falha

**Não abra uma issue pública.** Use o reporte privado do GitHub: aba **Security → Report a vulnerability** neste repositório ([link direto](https://github.com/cesarkali/Flow-Voice/security/advisories/new)). Só o mantenedor vê o relato.

Ajuda incluir:

- a versão do FlowVoice (em **Sobre**) e do Windows;
- o que um atacante consegue fazer e em que condições;
- os passos para reproduzir.

Você recebe uma resposta em até 7 dias. Depois da correção, a falha é descrita nas notas da versão, com crédito a quem reportou, se quiser.

## Versões cobertas

Só a versão mais recente recebe correções de segurança. O app se atualiza sozinho; se o seu não atualizou, baixe a última em [Releases](https://github.com/cesarkali/Flow-Voice/releases/latest).

## O que está no escopo

- vazamento de chaves de API (elas ficam no Gerenciador de Credenciais do Windows);
- áudio, transcrições ou texto selecionado indo para outro lugar além do provedor escolhido;
- execução de código ou elevação de privilégio pelo app, pelo instalador ou pelo atualizador;
- atualização falsa passando pela verificação de assinatura.

Problemas nos próprios provedores (Groq, Gemini, OpenAI…) devem ser reportados a eles.

---

**English:** please report vulnerabilities privately through **Security → Report a vulnerability** in this repository, not in a public issue. Only the latest version receives security fixes. You'll get a reply within 7 days.
