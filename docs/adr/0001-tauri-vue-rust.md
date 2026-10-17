# Tauri 2 com Vue 3 + TypeScript na interface e Rust no núcleo

O app precisa varrer discos inteiros rápido e ser leve, já que é um "otimizador". Escolhemos Tauri 2: a varredura, as Operações em arquivos e o acesso às APIs do Windows ficam no núcleo em Rust, e a interface é feita em Vue 3 + TypeScript, aproveitando a experiência do autor com TS. O desempenho percebido depende do núcleo e da virtualização das listas, não do framework de interface.

## Considered Options

- **C# + WPF/.NET**: desempenho próximo e ótimo acesso ao Windows, mas a interface em XAML seria nova para o autor e o app ficaria maior.
- **Electron**: familiar (Node), mas com cerca de 150 MB, contradiz a proposta de um app que otimiza o PC.
- **Python + Qt**: varredura lenta e distribuição ruim.
- **React no lugar de Vue**: desempenho equivalente; Vue foi escolhido por preferência do autor.
