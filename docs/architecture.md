# アーキテクチャ

llm-natsuki-bot (メカ夏稀) の全体構成をまとめたドキュメントです。図はすべて Mermaid で書かれているので、GitHub 上でそのまま描画されます。

## 1. システム全体図

`compose.yaml` で起動される 4 つのサービスと、外部サービスとの関係です。

```mermaid
flowchart LR
    subgraph platforms["チャットプラットフォーム"]
        mastodon[("Mastodon<br/>Streaming API (WebSocket / SSE)")]
        discord[("Discord<br/>Gateway / HTTP API")]
    end

    subgraph compose["docker compose"]
        bot["bot<br/>(lnb-server)"]
        api["api<br/>(lnb-admin-api)"]
        mathr["math-renderer<br/>(lnb-math-renderer / Hono + MathJax)"]
        valkey[("valkey<br/>(Redis 互換)")]
        sqlite[("SQLite<br/>./data 以下")]
    end

    subgraph external["外部 API"]
        llmapi["OpenAI 互換 API<br/>Chat Completion / Responses"]
        imgapi["画像生成 API<br/>/images/generations, /images/edits"]
        exapi["為替レート API"]
    end

    admin(["管理者 / 管理画面"])

    mastodon <--> bot
    discord <--> bot
    bot -- "会話の送信 / Tool Calling" --> llmapi
    bot -- "image_generator" --> imgapi
    bot -- "exchange_rate" --> exapi
    bot -- "LaTeX → PNG" --> mathr
    bot -- "会話ツリーの永続化" --> sqlite
    bot -- "リマインダージョブ" --> valkey

    admin -- "HTTP (JWT 認証)" --> api
    api -- "読み取り" --> sqlite
    api -- "読み取り" --> valkey
```

| サービス | 実装 | 役割 |
| --- | --- | --- |
| `bot` | `lnb-server` (Rust) | 本体。各プラットフォームのクライアントと会話処理エンジンを同居させて動かす |
| `api` | `lnb-admin-api` (Rust / axum) | 会話ログやリマインダーを参照する管理用 REST API |
| `math-renderer` | `lnb-math-renderer` (TypeScript / Hono + MathJax) | LaTeX 数式を PNG にレンダリングする HTTP サービス |
| `valkey` | Valkey | リマインダー (Shiyu) のジョブキュー |

## 2. クレート依存関係

Cargo workspace 内のクレートの依存関係です。`lnb-core` がトレイトとモデルを定義し、各クレートはその実装を提供する、というポート & アダプタ的な構成になっています。点線は `lnb-core` への依存です (サブグラフからの点線は、その中の全クレートが依存していることを表します)。

```mermaid
flowchart TB
    subgraph bins["バイナリ"]
        server["lnb-server"]
        adminapi["lnb-admin-api"]
    end

    subgraph clients["クライアント (LnbClient 実装)"]
        mastodon["lnb-mastodon-client"]
        discord["lnb-discord-client"]
    end

    subgraph infra["インフラ / 機能"]
        sqlite["lnb-persistence-sqlite"]
        redis["lnb-reminder-redis"]
        mathc["lnb-math-renderer-client"]
        policy["lnb-user-policy"]
    end

    subgraph standalone["独立クレート (lnb-core 非依存)"]
        rate["lnb-rate-limiter"]
        daily["lnb-daily-private"]
    end

    core["lnb-core<br/>(trait / model / error)"]

    server --> mastodon & discord
    server --> sqlite & redis & mathc & policy
    server --> rate & daily
    adminapi --> sqlite & redis

    mastodon --> mathc & policy
    discord --> policy

    bins -.-> core
    clients -.-> core
    sqlite & mathc & policy -.-> core
```

> `lnb-rate-limiter` / `lnb-daily-private` / `lnb-reminder-redis` は `lnb-core` に依存しない汎用クレートです。`lnb-reminder-redis` が `Reminder` トレイトを直接実装しているわけではなく、`lnb-server` 側の `Shiyu` がそれをラップしています。
>
> `xtask` は開発用タスクランナー (`cargo xtask`) なので図からは省いています。

### `lnb-core` が定義する主要なインターフェース

```mermaid
classDiagram
    direction LR

    class LnbServer {
        <<trait>>
        +new_conversation()
        +restore_conversation(context_key)
        +save_conversation(update, context_key)
        +process_conversation(message_ctx, id, messages)
    }
    class LnbClient {
        <<trait>>
        +execute()
    }
    class Llm {
        <<trait>>
        +send_conversation(conversation, descriptors) LlmUpdate
    }
    class Function {
        <<trait>>
        +get_descriptor() FunctionDescriptor
        +call(ctx, message_ctx, incomplete, tool_calling)
    }
    class Interception {
        <<trait>>
        +before_llm(context, incomplete) InterceptionStatus
    }
    class ConversationStorage {
        <<trait>>
        +fetch_content_by_id(id)
        +fetch_id_by_context_key(key)
        +insert(conversation, key)
        +update_if_current(expected, updated, key)
    }
    class Reminder {
        <<trait>>
        +register(context, remind, at)
        +remove(id)
    }
    class Remindable {
        <<trait>>
        +get_context()
        +remind(requester, update)
    }

    Natsuki ..|> LnbServer
    MastodonLnbClient ..|> LnbClient
    MastodonLnbClient ..|> Remindable
    DiscordLnbClient ..|> LnbClient
    Shiyu ..|> Reminder
    ChatCompletionBackend ..|> Llm
    ResponsesBackend ..|> Llm
    SqliteConversationStorage ..|> ConversationStorage
    MemoryConversationStorage ..|> ConversationStorage
    BangCommandInterception ..|> Interception

    Natsuki o-- ConversationStorage
    Natsuki o-- Llm : LlmCache 経由
    Natsuki o-- Function : FunctionStore 経由
    Natsuki o-- Interception
    MastodonLnbClient --> LnbServer
    DiscordLnbClient --> LnbServer
    Shiyu --> LnbServer
    Shiyu --> Remindable
```

## 3. `lnb-server` の内部構成

`main.rs` の `initialize_natsuki()` で組み立てられるコンポーネントです。

```mermaid
flowchart TB
    subgraph runtime["tokio JoinSet で並行実行されるサービス"]
        direction LR
        mc["MastodonLnbClient"]
        dc["DiscordLnbClient"]
        shiyu["Shiyu<br/>(リマインダーサービス)"]
    end

    subgraph natsuki["Natsuki (LnbServer 実装)"]
        entry(["process_conversation()"])
        rl["RateLimiter<br/>(lnb-rate-limiter)"]
        ic["Interceptions<br/>BangCommandInterception (!ping, !change ...)"]
        llmc["LlmCache<br/>モデル名 → ArcLlm を遅延初期化"]
        fs["FunctionStore"]
        st[("ConversationStorage<br/>Sqlite / Memory")]
        ctx["Context<br/>system role テンプレート / 日時プロバイダ"]

        entry --> rl --> ic --> llmc
        llmc -- "ToolCalling" --> fs
        entry <-- "fetch / update_if_current" --> st
        ctx -.-> entry
    end

    subgraph llms["LLM バックエンド (async-openai)"]
        direction LR
        cc["ChatCompletionBackend"]
        rs["ResponsesBackend"]
    end

    subgraph functions["Function (Tool)"]
        direction LR
        f1["SelfInfo"]
        f2["LocalInfo"]
        f3["ShiyuProvider<br/>(リマインダー登録)"]
        f4["ImageGenerator"]
        f5["MathRenderer"]
        f6["ExchangeRate"]
        f7["GetIllustUrl"]
        f8["DailyPrivate<br/>(lnb-daily-private)"]
    end

    mc & dc --> entry
    shiyu -- "リマインド会話の生成" --> entry
    shiyu -. "Remindable::remind" .-> mc

    llmc --> llms
    fs --> functions
    f3 -. "Reminder::register" .-> shiyu
```

`f4`〜`f8` は設定ファイルに対応するセクションがある場合のみ有効になるオプショナルな Function です。

## 4. メッセージ処理シーケンス

ユーザーがボットにメンションしてから返信が届くまでの流れです (`NatsukiInner::process_conversation`)。

```mermaid
sequenceDiagram
    autonumber
    actor User as ユーザー
    participant Client as Mastodon / Discord Client
    participant N as Natsuki
    participant RL as RateLimiter
    participant ST as ConversationStorage
    participant IC as Interception
    participant LLM as Llm (OpenAI 互換)
    participant FS as FunctionStore

    User->>Client: メンション付きの投稿
    Client->>Client: UserRole 判定 (lnb-user-policy)<br/>MessageContext 生成
    Client->>N: restore_conversation(context_key)
    alt 復元できない (新規スレッド / 未知のリプライ)
        Client->>N: new_conversation()
        N->>ST: insert(system message 付き Conversation)
        Client->>Client: 祖先投稿を辿って Message 列を再構築
    end
    Client->>N: process_conversation(ctx, id, messages)
    N->>RL: check(identity)
    N->>ST: fetch_content_by_id(id)

    loop 登録されている Interception (逆順)
        N->>IC: before_llm()
        alt Complete (例: !ping)
            IC-->>N: AssistantMessage
            N-->>Client: ConversationUpdate (LLM をスキップ)
        end
    end

    loop 最大 8 回 (MAX_CONVERSATION_LOOP)
        N->>LLM: send_conversation(conversation, function descriptors)
        alt ToolCalling
            LLM-->>N: tool_callings
            N->>FS: find_call()
            FS-->>N: FunctionResponse (+ 添付画像)
        else LengthCut
            LLM-->>N: 途中までの応答 (続行)
        else Finished / Filtered
            LLM-->>N: 最終応答
        end
    end

    N-->>Client: ConversationUpdate (エラー時はエラー文を返す一時的な Update)
    Client->>Client: 数式があれば math-renderer で PNG 化 (Mastodon)<br/>画像をアップロード
    Client->>User: 返信
    Client->>N: save_conversation(update, 返信投稿 ID の context_key)
    N->>ST: update_if_current() (楽観的排他制御)
```

## 5. リマインダー (Shiyu) の流れ

LLM が `ShiyuProvider` Function を呼ぶとジョブが Valkey に積まれ、時刻になると `Shiyu` がそれを取り出して会話を生成し、`Remindable` なクライアント経由で通知します。

```mermaid
sequenceDiagram
    autonumber
    participant LLM as Llm
    participant SP as ShiyuProvider (Function)
    participant Q as Valkey (lnb-reminder-redis)
    participant W as Shiyu Worker
    participant D as Shiyu Dispatcher
    participant N as Natsuki
    participant C as MastodonLnbClient (Remindable)

    LLM->>SP: tool calling (リマインダー登録)
    SP->>Q: enqueue_job(job, remind_at)

    loop 5 秒ごとにポーリング
        W->>Q: claim_jobs_until(now, lease 30 分)
        Q-->>W: 期限が来たジョブ
        W->>D: mpsc channel
    end

    D->>N: new_conversation() + process_conversation(system ctx)
    N-->>D: ConversationUpdate
    D->>C: remind(requester, update)
    alt 配送成功
        D->>Q: acknowledge_job(id)
    else 配送失敗 / 未知の context
        D->>Q: retry_job(id, now + 30 秒)
    end
```
