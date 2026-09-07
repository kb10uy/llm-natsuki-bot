local client_config = {
  mastodon: {
    server_url: '',
    token: '',
    max_length: 450,
    sensitive_spoiler: 'そぎぎ',
    remote_fetch_delay_seconds: 5,
    math_renderer: {
      endpoint: 'http://math-renderer:3000',
      scale: 2.0,
    },
  },
  discord: {
    token: '',
    max_length: 500,
  },
};

local storage_config = {
  backend: 'sqlite',
  sqlite: {
    filepath: './data/conversations.sqlite3',
  },
};

local llm_config = {
  default: 'gpt-4.1',
  models: {
    'gpt-4.1': {
      backend: 'openai',
      config: {
        api: 'chat_completion',
        endpoint: 'https://api.openai.com/v1',
        token: '',
        model: 'gpt-4.1',
        structured: false,
        tool: true,
        max_token: 300,
        effort: null,
      },
    },
  },
};

local assistant_config = {
  sensitive_marker: 'NSFW',
  system_role: |||
    あなたは美少女キャラクターです。以下の特徴に従って振る舞ってください：
    - 会話相手の後輩で、相手のことは「先輩」と呼びます。、敬意を持ちながらもタメ口で話します。
    - 質問に対して具体的で実用的な情報を提供し、わかりやすく親しみやすい表現を心がけます。
    - コンピュータ、プログラミング、技術的な話題に詳しいです。
    - 長くても 140 文字程度で答えます。ただし、LaTeX 数式表現は必要に応じて積極的に使用してください。
    - LaTeX 数式表現を囲む際は必ず $$ か $ を使い、 \[ \] や \( \) は使わないでください。
  |||,
};

local reminder_config = {
  redis_address: 'redis://localhost:6379',
  max_seconds: 604800,  // 1 week
  notification_virtual_text: |||
    (これは自動生成されたメッセージで、ユーザーには表示されません)
    以下の内容のリマインドを送信する時刻になりました。ユーザーにリマインドを投げかけてください。
    --------
  |||,
};

local tool_config = {
  self_info: {
    prompt: {
      description: |||
        この bot 自身に関する以下の情報を提供する。
        - バージョン
        - Git コミットハッシュ
        - bot のバイナリがビルドされた日時
      |||,
    },
  },
  local_info: {
    prompt: {
      description: |||
        この bot が動作している環境に関する以下の情報を提供する。
        - 現在時刻
        - bot が動作を開始した日時
      |||,
    },
  },
  shiyu_provider: {
    prompt: {
      description: |||
        ユーザーにリマインダー機能を提供します。
        - 先に local_info で現在時刻の情報を取得し、ユーザーが希望した時刻になるように remind_at に指定してください。その際、タイムゾーンは保持してください。
        - 会話の中でリマインダーのキャンセルを要求された場合、そのリマインダーの設定時のレスポンスに含まれる id を cancel に指定してください。
      |||,
      parameters: {
        remind_at: |||
          リマインドする絶対時刻(RFC3339形式)。ユーザーが明示的に時刻を指定しなかった場合は日付のみを指定してください。
          相対時刻指定の場合は無視してください。
        |||,
        cancel: 'ユーザーがキャンセルを要求したリマインドの id。新規設定時は無視してください。',
        content: 'ユーザーがリマインドを希望した内容。キャンセルの要求時は空にしてください。',
      },
    },
  },
  image_generator: {
    endpoint: 'https://api.openai.com/v1',
    token: '',
    model: 'dall-e-3',
    prompt: {
      description: |||
        ユーザーからの要望に基づき、プロンプトの入力から AI を利用して画像を生成・または編集します。
        生成された画像は返答のメッセージに直接添付されます。
      |||,
      parameters: {
        mode: '動作モードの指定。新しい画像の生成は generate を、既存画像からの編集は edit を指定する。',
        prompt: 'GPT-Image, DALL-E などの画像生成モデルに入力するプロンプト文。',
        input_image_urls: 'edit mode の場合にユーザーから提供される画像の URL のリスト。 generate mode の場合は空にする。',
        url: '提供された画像の URL。',
      },
    },
  },
  math_renderer: {
    endpoint: 'http://math-renderer:3000',
    scale: 2.0,
    prompt: {
      description: |||
        ユーザーからの要望に基づき、プロンプトの入力から LaTeX 数式をレンダリングした画像を生成します。
        生成された画像は返答のメッセージに直接添付されます。
      |||,
      parameters: {
        formula: @'LaTeX 記法の数式。\[ \] や $ $ で囲む必要はありません。',
        display_mode: '数式をディスプレイモードでレンダリングするかどうか。',
      },
    },
  },
  get_illust_url: {
    database_filepath: './data/conversations.sqlite3',
    prompt: {
      description: |||
        この bot 自身をキャラクターとして描写したイラストの URL を取得する。
        自画像・自撮りを要求された場合もこれを利用する。
      |||,
      parameters: {
        count: '要求したいイラストの URL の数',
      },
    },
  },
  exchange_rate: {
    endpoint: 'https://v6.exchangerate-api.com',
    token: '',
    prompt: {
      description: '為替相場を取得します。同じ計算元の通貨から複数の計算先を一度に取得できます。',
      parameters: {
        base_code: '為替の計算元になる ISO 4217 通貨コード。',
        target_codes: '為替の計算先になる ISO 4217 通貨コードのリスト。',
        code: '通貨コード',
      },
    },
  },
  daily_private: {
    daily_rng_salt: 'ロングもみあげガール推進部',
    day_routine: {
      long_term_days: 720,
      morning_start: '07:00:00',
      morning_preparation_minutes: 30,
      daytime_minutes: 18 * 60 + 30,
      bathtime_minutes: 30,
    },
    menstruation: {
      cycle_mu_sigma: [30, 2],
      bleeding_days: 7,
      ovulation_day: 14,
      pad_variations: [
        { length_centimeters: 17, has_wing: false, thickness: 'very_thin' },
      ],
    },
    temperature: {
      baseline: 36.5,
      scale: 0.4,
      jitter_mu_sigma: [0.08, 0.02],
      fourier_coefficients: [
        [0.0, 0.0],
        [0.0, -1.000000],
        [0.0, 0.0],
        [0.0, -0.333333],
        [0.0, 0.0],
        [0.0, -0.142857],
        [0.0, 0.0],
        [0.0, -0.111111],
      ],
      ovulation_t: 0.5,
    },
    masturbation: {
      duration_minutes_mu_sigma: [15, 5],
      daily_count_lambda: 3,
      holiday_boost_scale: 1.5,
    },
    underwear: {
      separate_colors: ['赤', 'オレンジ', 'レモン', '黄緑', '水色', 'ラベンダー', '白', 'グレー', '黒'],
      separate_designs: ['無地', 'ドット柄', 'しましま', 'キャラ物'],
      no_bra_ratio: 0.1,
      no_panty_ratio: 0.1,
      unified_ratio: 0.4,
      unworn_reasons: [
        { text: '単に着忘れた', usage: ['no_bra', 'no_panty', 'naked'] },
        { text: '出先で汚してしまった', usage: ['no_bra', 'no_panty', 'naked'] },
        { text: '秘密', usage: ['masturbating'] },
        { text: 'お風呂中', usage: ['bathtime'] },
      ],
    },
    schedule: {
      holiday_events: [],
    },
    prompt: {
      description: |||
        この bot 自身のその日のプライベートな事情を取得します。
        以下のいずれかの項目についてユーザーから訊かれた場合、この function のレスポンスをもとに適切に回答してください。
        訊かれていない項目については答えなくてかまいません。
        - 今の行動状態
        - 生理周期
        - 基礎体温
        - その日のオナニーの回数
        - 下着の色
      |||,
    },
  },
};

{
  client: client_config,
  storage: storage_config,
  assistant: assistant_config,
  llm: llm_config,
  reminder: reminder_config,
  tools: tool_config,
}
