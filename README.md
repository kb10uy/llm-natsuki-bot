# LLM Natsuki bot
OpenAI API とかを使っておしゃべりする夏稀

## ビルド時の注意
* ビルド時にコンテナに .git を入れてないので `GIT_COMMIT_HASH` 変数を外から手動で渡す必要がある
    - `docker compose build --build-arg "GIT_COMMIT_HASH=$(git rev-parse HEAD)"`

## Links
* [DeepWiki: kb10uy/llm-natsuki-bot](https://deepwiki.com/kb10uy/llm-natsuki-bot)
