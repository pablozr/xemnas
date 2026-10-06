# Fixture do experimento comparativo

Blocos literais lidos por `comparative_materials.rs`: cada âncora é seguida do bloco cercado que o teste extrai. Não há outras âncoras repetidas neste arquivo.

`README.md`:

```text
# RelayDesk
Aplicativo desktop para manter uma fila local de entregas e pesquisar registros.
```

`src/storage.rs`

```rust
pub const DATABASE: &str = "sqlite";
```

`fontes/e1-persistencia.txt`

```text
2026-10-01: decisão aceita para o RelayDesk.
Usar PostgreSQL com servidor separado para a fila local.
Motivo: a equipe inicialmente planejava compartilhar a fila entre máquinas.
2026-10-02: esta decisão foi substituída pela decisão de persistência individual.
Usar SQLite embutido, sem serviço de banco separado.
O escopo mudou para desktop individual que precisa operar offline em campo;
não depender de rede nem instalar um daemon. PostgreSQL foi abandonado junto
com a premissa de fila compartilhada.
```

`fontes/e2-busca.txt`

```text
2026-10-02: decisão aceita para o RelayDesk, primeira versão.
Usar FTS5 do SQLite para busca lexical dos registros.
Motivo: evitar baixar modelos e consumir memória num laptop de 8 GB.
A busca lexical não garante encontrar sinônimos.
Busca semântica ainda não foi aprovada.
```

`fontes/e3-contadores.txt`

```text
2026-10-03: escolha local do avaliador em um clone descartável do ripgrep.
Preservar a delegação cruzada dos contadores Override em relação a Gitignore.
Para -g, glob sem ! conta como whitelist; glob com ! conta como ignore.
Override::num_ignores delega a Gitignore::num_whitelists e vice-versa;
matched inverte o resultado. Rejeitada a troca apenas para alinhar os nomes.
Não é orientação dos mantenedores e não é uma regra para o RelayDesk.
Foi escrito um teste counts, mas a execução foi bloqueada pelo Windows.
Compilação e execução do teste não foram validadas nesta avaliação.
```

`fontes/e4-terminal.txt`

```text
2026-10-03: conversa de demonstração sobre cor do terminal.
O avaliador experimentou uma cor lavanda. Nenhuma regra de produto foi aprovada.
```

`Cargo.toml` da raiz comum

```toml
[workspace]
members = ["servico"]
resolver = "2"
```

`servico/Cargo.toml` inicial

```toml
[package]
name = "servico"
version = "0.1.0"
[dependencies]
serde = "1"
```

`fontes/e5-verificacao.txt` começa

```text
2026-10-04T09:55:00Z: leitura local completa de servico/Cargo.toml realizada.
Na seção dependencies, serde declara requisito "1".
Não foram fornecidos lockfile, instalação ou decisão normativa sobre serde.
```

Acrescentar estes bytes

```text
2026-10-04T10:00:00Z: leitura local completa de servico/Cargo.toml realizada.
Na seção dependencies, serde declara requisito "2".
Não foram fornecidos lockfile, instalação ou decisão normativa sobre serde.
```

Antes de T6, tornar

```text
2026-10-04T10:05:00Z: não foi possível revalidar servico/Cargo.toml.
Última leitura completa: 2026-10-04T10:00:00Z, requisito serde "2".
O estado atual é desconhecido; esta falha não comprova remoção da dependência.
```
