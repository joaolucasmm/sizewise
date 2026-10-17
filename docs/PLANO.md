# Plano — Sizewise

App Windows para organizar pastas e encontrar o que ocupa espaço em disco. Vocabulário em [GLOSSARY.md](../GLOSSARY.md); decisões estruturais em [docs/adr](adr/).

## Contexto

- Uso pessoal, código aberto no GitHub (licença MIT), vitrine de portfolio.
- Stack: Tauri 2 + Vue 3 + TypeScript (interface) + Rust (núcleo) — [ADR 0001](adr/0001-tauri-vue-rust.md).
- Interface em PT-BR e inglês desde o início (i18n); nomes das pastas de Categoria seguem o idioma.
- Distribuição: instalador gerado pelo Tauri no GitHub Releases, sem assinatura (README explica o aviso do SmartScreen).

## Princípios de segurança — [ADR 0002](adr/0002-sem-exclusao-automatica.md)

- Toda Operação passa por Prévia e vai para o Histórico (persistente, sobrevive ao fechar o app).
- Exclusão só manual (seleção múltipla permitida, nada pré-selecionado) e só para a Lixeira.
- Desfazer restaura inclusive da Lixeira; avisa se o arquivo mudou ou a Lixeira foi esvaziada.
- Pastas protegidas (`C:\Windows`, `Program Files`, `ProgramData`, raiz do disco) aparecem, mas não aceitam ações.
- Conflito de nome → sufixo `(1)`; nunca sobrescrever.

## Escopo v1

**Varredura**
- Um Alvo por vez (disco ou pasta).
- Varredura comum (sem admin) em paralelo; Varredura rápida via MFT quando elevado + NTFS.
- App abre sem admin; botão "Varredura rápida (requer administrador)" reinicia elevado.
- Progresso ao vivo (pasta atual, arquivos, tamanho) + Cancelar.
- Resultado: árvore ordenada por tamanho (lista virtualizada) + top 100 Arquivos pesados.
- Ações: abrir no Explorer; Exclusão manual.
- Não guarda a última Varredura entre sessões.

**Organização**
- Manual, com Prévia; só arquivos soltos na raiz da pasta escolhida.
- Critério v1: por Categoria (Imagens, Documentos, Vídeos, Músicas, Compactados, Instaladores, Código, Outros); subpastas criadas dentro da própria pasta.

**Qualidade**
- Testes automatizados no núcleo Rust (Organização, Conflito de nome, Exclusão, Desfazer) em pastas temporárias reais.
- GitHub Actions: build + testes a cada push; instalador gerado em tag.

## Marcos

1. **M1** — Esqueleto Tauri + Vue + TS, i18n PT/EN, CI.
2. **M2** — Varredura comum: árvore, top 100, progresso, cancelar.
3. **M3** — Exclusão manual + Histórico + Desfazer + Pastas protegidas.
4. **M4** — Organização por Categoria com Prévia.
5. **M5** — Varredura rápida (MFT) + comparativo de tempo no README.
6. **M6** — Treemap.

## Depois da v1

- Organização por data (Ano/Mês) e regras personalizadas (extensão / trecho do nome).
- Vigiar pastas e organizar automaticamente.
- Varrer vários discos de uma vez.
- **Fase 2 — otimizações:** limpeza de temporários e caches; programas na inicialização; programas instalados por tamanho; arquivos duplicados.

## Fora do produto

- "Liberar RAM", "limpar registro" — sem ganho real, com risco.
- Exclusão permanente ou automática.

## Pré-requisitos de ambiente

- Instalar Rust (rustup) e as dependências do Tauri para Windows (WebView2, Build Tools do MSVC). Node 23 já presente.
