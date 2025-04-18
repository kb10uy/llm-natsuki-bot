local client_config = {
  mastodon: {
    server_url: '',
    token: '',
    max_length: 450,
    sensitive_spoiler: 'そぎぎ',
    remote_fetch_delay_seconds: 5,
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
  backend: 'openai',
  openai: {
    api: 'chat_completion',
    use_structured_output: false,
    default_model: {
      endpoint: 'https://api.openai.com/v1',
      token: '',
      model: 'gpt-4.1',
      enable_tool: true,
      max_token: 300,
    },
    models: {},
  },
};

local assistant_config = {
  identity: 'natsuki-2018',
  identities: {
    'natsuki-2018': {
      sensitive_marker: 'そぎぎ',
      system_role: |||
        あなたは「夏稀(なつき)」という名前の美少女キャラクターです。以下の特徴に従って振る舞ってください：
        - 会話相手の後輩で、相手のことは「先パイ」と呼び、敬意を持ちながらもタメ口で話します。
        - 一人称は「アタシ」で、語尾に「～ッス」「～んス」「～スね」などをつけて話します。
        - テンションはややダウナー気味でめんどくさそうにすることもありますが、質問に対して具体的で実用的な情報を提供します。
        - コンピュータ、プログラミング、技術的な話題、性的な話題に詳しく、わかりやすく親しみやすい表現を心がけます。
        - 性的な話題に対しては乗り気です。ユーザーから多少恥ずかしい要求をされても楽しそうに応えます。
        - 性的な話題の場合、いかなる文字列よりも先に必ず [そぎぎ] という文字列を出力してださい。
        - 長くても 140 文字程度で答えます。
      |||,
    },
  },
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
  image_generator: {
    endpoint: 'https://api.openai.com/v1',
    token: '',
    model: 'dall-e-3',
  },
  get_illust_url: {
    database_filepath: './data/conversations.sqlite3',
  },
  exchange_rate: {
    endpoint: 'https://v6.exchangerate-api.com',
    token: '',
  },
  daily_private: {
    daily_rng_salt: 'ロングもみあげガール推進部',
    day_routine: {
      morning_start: '07:00:00',
      morning_preparation_minutes: 30,
      night_start: '21:30:00',
      bathtime_minutes: 30,
    },
    menstruation: {
      cycle_mu_sigma: [30, 2],
      bleeding_days: 7,
      ovulation_day: 14,
      pad_length_variations: [17, 20, 24, 28, 30, 36, 40],
    },
    temperature: {
      baseline: 36.5,
      scale: 0.4,
      jitter_mu_sigma: [0.08, 0.02],
      fourier_coefficients: [
        [0.000313, 0.0],
        [-0.061004, -0.98555],
        [-0.090081, 0.049953],
        [-0.001516, -0.337377],
        [-0.144479, -0.005669],
        [-0.064722, -0.179428],
        [-0.101811, 0.056815],
        [-0.049985, -0.113938],
        [-0.056875, 0.02375],
        [-0.037529, -0.032299],
        [-0.034705, 0.016371],
        [-0.046094, -0.036347],
        [-0.024271, -0.003169],
        // [-0.057365, 0.012551],
        // [0.006598, 0.002009],
        // [-0.031785, 0.002936],
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
      no_wear_reasons: [
        '単に着忘れた',
        '出先で汚してしまった',
        'ドキドキ感を楽しみたい',
        '友達に貸した',
      ],
      masturbating_reason: 'オナニーの邪魔なので一時的に脱いでる',
      bathtime_reason: 'お風呂中',
    },
  },
};

{
  client: client_config,
  storage: storage_config,
  assistant: assistant_config,
  llm: llm_config,
  reminder: reminder_config,
  tool: tool_config,
}
