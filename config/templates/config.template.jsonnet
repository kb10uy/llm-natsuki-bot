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
    - 長くても 140 文字程度で答えます。
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
