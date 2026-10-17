# Sizewise

App para Windows que organiza pastas e mostra onde está o espaço ocupado em disco, sempre com o usuário no controle de cada alteração.

## Organização

**Organização**:
Mover os arquivos soltos na raiz de uma pasta escolhida para subpastas, segundo um critério. Nunca apaga arquivos nem mexe em subpastas existentes.
_Avoid_: Limpeza, arrumação

**Categoria**:
Grupo de tipos de arquivo (Imagens, Documentos, Vídeos, Músicas, Compactados, Instaladores, Código, Outros) que vira uma subpasta na Organização por tipo.
_Avoid_: Tipo, grupo

**Conflito de nome**:
Quando o destino já contém um arquivo com o mesmo nome; resolvido com sufixo numérico, ex.: `foto (1).jpg`. Nunca há sobrescrita.

## Varredura

**Varredura**:
Medição do espaço ocupado por cada arquivo e pasta de um Alvo.
_Avoid_: Scan, análise

**Alvo**:
O disco ou a pasta que uma Varredura mede. Uma Varredura tem exatamente um Alvo.

**Varredura rápida**:
Varredura que lê o índice do sistema de arquivos NTFS de uma vez; exige administrador.

**Varredura comum**:
Varredura que percorre as pastas uma a uma; não exige administrador.

**Arquivo pesado**:
Arquivo entre os maiores resultados de uma Varredura.

**Pasta protegida**:
Pasta do sistema (ex.: `C:\Windows`, `Program Files`, `ProgramData`, raiz do disco) que aparece na Varredura mas não aceita Organização nem Exclusão.

## Operações

**Operação**:
Ação confirmada pelo usuário que altera arquivos (mover, renomear, enviar para a Lixeira). Toda Operação passa por Prévia e fica no Histórico.

**Prévia**:
Lista do que uma Operação fará, mostrada antes de executá-la.

**Histórico**:
Registro persistente das Operações executadas, que sobrevive ao fechar o app.

**Desfazer**:
Reverter uma Operação do Histórico, devolvendo os arquivos ao estado anterior, inclusive restaurando da Lixeira.

**Exclusão**:
Envio para a Lixeira do Windows de arquivos que o próprio usuário selecionou. O app nunca apaga permanentemente nem sugere o que apagar.
_Avoid_: Deletar, limpar, remover
