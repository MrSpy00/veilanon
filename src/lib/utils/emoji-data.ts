/**
 * veilanon — emoji data: kategorili emoji seti + favori yardımcıları.
 * Favoriler localStorage'da tutulur; arama ad/kategori kelimeleriyle çalışır.
 */

export interface EmojiEntry {
  e: string;
  /** Arama anahtar kelimeleri (küçük harf, İngilizce adlar). */
  k: string;
}

export interface EmojiCategory {
  id: string;
  label: string;
  emojis: EmojiEntry[];
}

export const EMOJI_CATEGORIES: EmojiCategory[] = [
  {
    id: 'smileys',
    label: 'İfadeler',
    emojis: [
      { e: '😀', k: 'grinning smiley yuz' }, { e: '😄', k: 'smile yuz' }, { e: '😁', k: 'grin teeth' },
      { e: '😆', k: 'laugh happy' }, { e: '😂', k: 'joy laugh tears' }, { e: '🤣', k: 'rofl laugh' },
      { e: '😊', k: 'blush smile' }, { e: '😇', k: 'innocent angel' }, { e: '🙂', k: 'slight smile' },
      { e: '😉', k: 'wink' }, { e: '😍', k: 'heart eyes love' }, { e: '🥰', k: 'smiling hearts' },
      { e: '😘', k: 'kiss blow' }, { e: '😋', k: 'yum tasty' }, { e: '😎', k: 'cool sunglasses' },
      { e: '🤓', k: 'nerd glasses' }, { e: '🥳', k: 'party celebrate' }, { e: '🤩', k: 'star eyes wow' },
      { e: '😜', k: 'wink tongue silly' }, { e: '🤪', k: 'crazy zany' }, { e: '🤔', k: 'thinking hmm' },
      { e: '🤗', k: 'hug' }, { e: '🤭', k: 'shush secret' }, { e: '😐', k: 'neutral face' },
      { e: '😴', k: 'sleeping tired' }, { e: '🥱', k: 'yawn bored' }, { e: '😪', k: 'sleepy' },
      { e: '😢', k: 'cry tears sad' }, { e: '😭', k: 'sob crying' }, { e: '😤', k: 'steam frustrated' },
      { e: '😡', k: 'angry red' }, { e: '🤬', k: 'cursing swear' }, { e: '😱', k: 'scream shocked' },
      { e: '😨', k: 'fear scared' }, { e: '😰', k: 'anxious sweat' }, { e: '😥', k: 'relieved sad' },
      { e: '🤯', k: 'mind blown head' }, { e: '😳', k: 'flushed embarassed' }, { e: '🥺', k: 'pleading puppy' },
      { e: '😷', k: 'mask sick' }, { e: '🤒', k: 'thermometer ill' }, { e: '🤕', k: 'bandage hurt' },
      { e: '🥴', k: 'dizzy woozy' }, { e: '😵', k: 'dizzy confused' }, { e: '🤠', k: 'cowboy hat' },
      { e: '🥸', k: 'disguise fake' }, { e: '😈', k: 'smiling devil evil' }, { e: '👿', k: 'angry devil' },
      { e: '👻', k: 'ghost' }, { e: '💀', k: 'skull dead' }, { e: '🤖', k: 'robot' },
      { e: '👽', k: 'alien' }, { e: '😺', k: 'cat smile' }, { e: '🙈', k: 'see no evil monkey' },
      { e: '🙉', k: 'hear no evil' }, { e: '🙊', k: 'speak no evil' }, { e: '💩', k: 'poop' },
      { e: '✨', k: 'sparkles magic' }, { e: '⭐', k: 'star' }, { e: '🌟', k: 'glowing star' },
      { e: '💫', k: 'dizzy star' }, { e: '🔥', k: 'fire hot' }, { e: '💥', k: 'boom collision' },
      { e: '💯', k: 'hundred perfect' }, { e: '❤️', k: 'heart love' }, { e: '🧡', k: 'orange heart' },
      { e: '💛', k: 'yellow heart' }, { e: '💚', k: 'green heart' }, { e: '💙', k: 'blue heart' },
      { e: '💜', k: 'purple heart' }, { e: '🖤', k: 'black heart' }, { e: '🤍', k: 'white heart' },
      { e: '💔', k: 'broken heart' }, { e: '💖', k: 'sparkling heart' }, { e: '💘', k: 'heart arrow cupid' },
      { e: '💕', k: 'two hearts' }, { e: '👍', k: 'thumbs up like' }, { e: '👎', k: 'thumbs down' },
      { e: '👏', k: 'clap applause' }, { e: '🙏', k: 'pray please thanks' }, { e: '🤝', k: 'handshake deal' },
      { e: '💪', k: 'muscle strong' }, { e: '🫶', k: 'heart hands' }, { e: '👌', k: 'ok perfect' },
      { e: '🤌', k: 'pinched fingers' }, { e: '✌️', k: 'victory peace' }, { e: '🤞', k: 'crossed fingers luck' },
      { e: '🫡', k: 'salute respect' }, { e: '🤙', k: 'call me shaka' }, { e: '👊', k: 'fist punch' },
      { e: '✊', k: 'raised fist power' }, { e: '🖐️', k: 'hand raised' }, { e: '👋', k: 'wave hello bye' },
      { e: '🤚', k: 'raised backhand' }, { e: '🫰', k: 'fingers heart' }, { e: '💅', k: 'nail polish fancy' },
      { e: '🤳', k: 'selfie' }, { e: '👀', k: 'eyes watch' }, { e: '👁️', k: 'eye' },
      { e: '🧠', k: 'brain smart' }, { e: '🫀', k: 'anatomical heart' }, { e: '🦷', k: 'tooth' },
      { e: '🦴', k: 'bone' }, { e: '🗣️', k: 'speaking head talk' },
    ],
  },
  {
    id: 'people',
    label: 'Kişiler',
    emojis: [
      { e: '😀', k: '' }, { e: '👋', k: '' }, { e: '🧑‍🤝‍🧑', k: 'people friends' },
      { e: '👨‍👩‍👧‍👦', k: 'family' }, { e: '🤵', k: 'groom suit' }, { e: '👰', k: 'bride veil' },
      { e: '👮', k: 'police officer' }, { e: '🕵️', k: 'detective spy' }, { e: '👩‍⚕️', k: 'doctor nurse' },
      { e: '👨‍🔧', k: 'mechanic worker' }, { e: '👨‍🎨', k: 'artist paint' }, { e: '👩‍🎓', k: 'student graduate' },
      { e: '👨‍🏫', k: 'teacher' }, { e: '👨‍💻', k: 'technologist coder' }, { e: '👩‍🚀', k: 'astronaut' },
      { e: '🧙', k: 'mage wizard' }, { e: '🧛', k: 'vampire' }, { e: '🧟', k: 'zombie' },
      { e: '🦸', k: 'superhero' }, { e: '🦹', k: 'supervillain' }, { e: '🧚', k: 'fairy' },
      { e: '🧜‍♀️', k: 'mermaid' }, { e: '🧝', k: 'elf' }, { e: '🗿', k: 'moai statue' },
    ],
  },
  {
    id: 'animals',
    label: 'Hayvanlar',
    emojis: [
      { e: '🐶', k: 'dog puppy' }, { e: '🐱', k: 'cat kitty' }, { e: '🐭', k: 'mouse' },
      { e: '🐹', k: 'hamster' }, { e: '🐰', k: 'rabbit bunny' }, { e: '🦊', k: 'fox' },
      { e: '🐻', k: 'bear' }, { e: '🐼', k: 'panda' }, { e: '🐨', k: 'koala' },
      { e: '🐯', k: 'tiger' }, { e: '🦁', k: 'lion' }, { e: '🐮', k: 'cow' },
      { e: '🐷', k: 'pig' }, { e: '🐸', k: 'frog' }, { e: '🐵', k: 'monkey face' },
      { e: '🐔', k: 'chicken' }, { e: '🐧', k: 'penguin' }, { e: '🐦', k: 'bird' },
      { e: '🦄', k: 'unicorn' }, { e: '🐴', k: 'horse' }, { e: '🐺', k: 'wolf' },
      { e: '🐝', k: 'bee honey' }, { e: '🦋', k: 'butterfly' }, { e: '🐢', k: 'turtle' },
      { e: '🐍', k: 'snake' }, { e: '🐙', k: 'octopus' }, { e: '🦀', k: 'crab' },
      { e: '🐳', k: 'whale' }, { e: '🐬', k: 'dolphin' }, { e: '🦈', k: 'shark' },
      { e: '🐲', k: 'dragon' }, { e: '🦖', k: 't-rex dino' }, { e: '🦕', k: 'sauropod dino' },
      { e: '🐌', k: 'snail slow' }, { e: '🦉', k: 'owl wise' }, { e: '🦜', k: 'parrot' },
      { e: '🐿️', k: 'squirrel' }, { e: '🦔', k: 'hedgehog' }, { e: '🐾', k: 'paw prints' },
    ],
  },
  {
    id: 'food',
    label: 'Yiyecek',
    emojis: [
      { e: '🍎', k: 'apple' }, { e: '🍌', k: 'banana' }, { e: '🍉', k: 'watermelon' },
      { e: '🍇', k: 'grapes' }, { e: '🍓', k: 'strawberry' }, { e: '🍒', k: 'cherry' },
      { e: '🍍', k: 'pineapple' }, { e: '🥑', k: 'avocado' }, { e: '🍕', k: 'pizza' },
      { e: '🍔', k: 'burger' }, { e: '🍟', k: 'fries' }, { e: '🌭', k: 'hotdog' },
      { e: '🌮', k: 'taco' }, { e: '🍗', k: 'chicken leg' }, { e: '🥩', k: 'steak meat' },
      { e: '🍜', k: 'noodles ramen' }, { e: '🍣', k: 'sushi' }, { e: '🍦', k: 'ice cream' },
      { e: '🍩', k: 'donut' }, { e: '🍪', k: 'cookie' }, { e: '🎂', k: 'birthday cake' },
      { e: '🍫', k: 'chocolate' }, { e: '☕', k: 'coffee tea' }, { e: '🍵', k: 'green tea' },
      { e: '🧋', k: 'bubble tea' }, { e: '🍺', k: 'beer' }, { e: '🍷', k: 'wine' },
      { e: '🥂', k: 'cheers toast' }, { e: '🍹', k: 'cocktail juice' }, { e: '🥨', k: 'pretzel' },
      { e: '🧀', k: 'cheese' }, { e: '🥚', k: 'egg' }, { e: '🍳', k: 'cooking egg' },
    ],
  },
  {
    id: 'activity',
    label: 'Aktiviteler',
    emojis: [
      { e: '⚽', k: 'soccer football' }, { e: '🏀', k: 'basketball' }, { e: '🏈', k: 'american football' },
      { e: '⚾', k: 'baseball' }, { e: '🎾', k: 'tennis' }, { e: '🏐', k: 'volleyball' },
      { e: '🎱', k: 'billiards pool' }, { e: '🏓', k: 'ping pong' }, { e: '🏸', k: 'badminton' },
      { e: '🥊', k: 'boxing' }, { e: '🥋', k: 'martial arts' }, { e: '🎮', k: 'video game' },
      { e: '🕹️', k: 'joystick' }, { e: '🎲', k: 'dice game' }, { e: '🎯', k: 'target dart' },
      { e: '🎳', k: 'bowling' }, { e: '🏆', k: 'trophy win' }, { e: '🥇', k: 'gold medal' },
      { e: '🥈', k: 'silver medal' }, { e: '🥉', k: 'bronze medal' }, { e: '🏅', k: 'sports medal' },
      { e: '🎨', k: 'art palette' }, { e: '🎭', k: 'theater drama' }, { e: '🎤', k: 'mic sing karaoke' },
      { e: '🎧', k: 'headphone music' }, { e: '🎸', k: 'guitar' }, { e: '🎹', k: 'piano keyboard' },
      { e: '🥁', k: 'drum' }, { e: '🎺', k: 'trumpet' }, { e: '🎻', k: 'violin' },
      { e: '🎬', k: 'movie clapper' }, { e: '🎪', k: 'circus' }, { e: '🎢', k: 'roller coaster' },
      { e: '🚴', k: 'cycling bike' }, { e: '🏊', k: 'swimming' }, { e: '🧗', k: 'climbing' },
      { e: '🏋️', k: 'weightlifting gym' }, { e: '🎣', k: 'fishing' }, { e: '🏹', k: 'archery' },
      { e: '🧘', k: 'meditation yoga' }, { e: '🏄', k: 'surfing' }, { e: '⛸️', k: 'ice skate' },
      { e: '🛹', k: 'skateboard' }, { e: '🛼', k: 'roller skate' },
    ],
  },
  {
    id: 'travel',
    label: 'Seyahat',
    emojis: [
      { e: '🚗', k: 'car' }, { e: '🚕', k: 'taxi' }, { e: '🚙', k: 'suv' },
      { e: '🚌', k: 'bus' }, { e: '🚎', k: 'trolleybus' }, { e: '🏎️', k: 'race car' },
      { e: '🚓', k: 'police car' }, { e: '🚑', k: 'ambulance' }, { e: '🚒', k: 'fire truck' },
      { e: '🚜', k: 'tractor' }, { e: '🏍️', k: 'motorcycle' }, { e: '🛵', k: 'scooter' },
      { e: '🚲', k: 'bicycle' }, { e: '🚂', k: 'train' }, { e: '🚄', k: 'high speed train' },
      { e: '🚀', k: 'rocket launch' }, { e: '✈️', k: 'airplane flight' }, { e: '🚁', k: 'helicopter' },
      { e: '⛵', k: 'sailboat' }, { e: '🚤', k: 'speedboat' }, { e: '🛸', k: 'ufo flying saucer' },
      { e: '🗺️', k: 'map world' }, { e: '🌍', k: 'earth globe' }, { e: '🏔️', k: 'mountain' },
      { e: '🏖️', k: 'beach' }, { e: '🏝️', k: 'island' }, { e: '🌋', k: 'volcano' },
      { e: '🗽', k: 'statue liberty' }, { e: '🗼', k: 'tokyo tower' }, { e: '🏰', k: 'castle' },
      { e: '🏯', k: 'japanese castle' }, { e: '🎡', k: 'ferris wheel' }, { e: '🌅', k: 'sunrise' },
      { e: '🌄', k: 'sunrise mountain' }, { e: '🌃', k: 'night city' }, { e: '🌌', k: 'milky way night' },
      { e: '🌈', k: 'rainbow' }, { e: '⛈️', k: 'thunder storm' }, { e: '❄️', k: 'snowflake cold' },
      { e: '☀️', k: 'sunny' }, { e: '🌙', k: 'moon night' }, { e: '🪐', k: 'saturn planet' },
    ],
  },
  {
    id: 'objects',
    label: 'Nesneler',
    emojis: [
      { e: '⌚', k: 'watch time' }, { e: '📱', k: 'smartphone phone' }, { e: '💻', k: 'laptop computer' },
      { e: '🖥️', k: 'desktop monitor' }, { e: '🖱️', k: 'mouse' }, { e: '⌨️', k: 'keyboard' },
      { e: '📷', k: 'camera photo' }, { e: '🎥', k: 'video camera' }, { e: '🎞️', k: 'film frames' },
      { e: '📺', k: 'tv television' }, { e: '📻', k: 'radio' }, { e: '🔊', k: 'speaker sound' },
      { e: '🔋', k: 'battery' }, { e: '🔌', k: 'plug' }, { e: '💡', k: 'bulb idea' },
      { e: '🔦', k: 'flashlight' }, { e: '🕯️', k: 'candle' }, { e: '💰', k: 'money bag cash' },
      { e: '💎', k: 'gem diamond' }, { e: '🪙', k: 'coin' }, { e: '📚', k: 'books' },
      { e: '📖', k: 'open book' }, { e: '📝', k: 'memo note' }, { e: '✏️', k: 'pencil' },
      { e: '🖊️', k: 'pen' }, { e: '📌', k: 'pushpin' }, { e: '📎', k: 'paperclip' },
      { e: '✂️', k: 'scissors' }, { e: '🔑', k: 'key' }, { e: '🔒', k: 'lock' },
      { e: '🔓', k: 'unlock' }, { e: '🔨', k: 'hammer' }, { e: '🪓', k: 'axe' },
      { e: '🔧', k: 'wrench tool' }, { e: '🔩', k: 'nut bolt' }, { e: '⚙️', k: 'gear settings' },
      { e: '🧲', k: 'magnet' }, { e: '🧪', k: 'test tube science' }, { e: '🔬', k: 'microscope' },
      { e: '🔭', k: 'telescope' }, { e: '📡', k: 'satellite antenna' }, { e: '💉', k: 'syringe' },
      { e: '💊', k: 'pill medicine' }, { e: '🧬', k: 'dna' }, { e: '🪄', k: 'magic wand' },
      { e: '🎁', k: 'gift present' }, { e: '🎈', k: 'balloon' }, { e: '🎊', k: 'confetti party' },
      { e: '🪆', k: 'nesting dolls' }, { e: '📦', k: 'package box' },
    ],
  },
  {
    id: 'symbols',
    label: 'Semboller',
    emojis: [
      { e: '✅', k: 'check yes done' }, { e: '❌', k: 'cross no' }, { e: '❓', k: 'question' },
      { e: '❗', k: 'exclamation' }, { e: '⚠️', k: 'warning' }, { e: '🚫', k: 'prohibited stop' },
      { e: '💲', k: 'dollar money' }, { e: '♻️', k: 'recycle' }, { e: '🔴', k: 'red circle' },
      { e: '🟠', k: 'orange circle' }, { e: '🟡', k: 'yellow circle' }, { e: '🟢', k: 'green circle' },
      { e: '🔵', k: 'blue circle' }, { e: '🟣', k: 'purple circle' }, { e: '⚫', k: 'black circle' },
      { e: '⚪', k: 'white circle' }, { e: '🔺', k: 'red triangle up' }, { e: '🔻', k: 'red triangle down' },
      { e: '⭐', k: 'star' }, { e: '🌠', k: 'shooting star' }, { e: '🏁', k: 'checkered flag' },
      { e: '🚩', k: 'red flag' }, { e: '🔞', k: 'nsfw 18+' }, { e: '♠️', k: 'spade' },
      { e: '♥️', k: 'heart suit' }, { e: '♦️', k: 'diamond suit' }, { e: '♣️', k: 'club' },
      { e: '🆕', k: 'new fresh' }, { e: '🆗', k: 'ok button' }, { e: '🆒', k: 'cool button' },
      { e: '🆓', k: 'free button' }, { e: '🔝', k: 'top' }, { e: '💤', k: 'zzz sleep' },
      { e: '💢', k: 'anger symbol' }, { e: '♨️', k: 'hot springs' }, { e: '🔔', k: 'bell notification' },
      { e: '🔕', k: 'bell muted' }, { e: '📢', k: 'loudspeaker announce' }, { e: '📣', k: 'megaphone shout' },
      { e: '🕐', k: 'one oclock time' }, { e: '🕒', k: 'three oclock' }, { e: '🕕', k: 'six oclock' },
      { e: '🕘', k: 'nine oclock' }, { e: '🕛', k: 'twelve oclock' },
    ],
  },
];

const FAV_KEY = 'veilanon-fav-emojis';

export function loadFavoriteEmojis(): string[] {
  try {
    const raw = localStorage.getItem(FAV_KEY);
    const parsed = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? parsed.filter(e => typeof e === 'string') : [];
  } catch {
    return [];
  }
}

export function saveFavoriteEmoji(emoji: string) {
  const favs = loadFavoriteEmojis();
  if (!favs.includes(emoji)) {
    favs.unshift(emoji);
    localStorage.setItem(FAV_KEY, JSON.stringify(favs.slice(0, 36)));
  }
}

export function removeFavoriteEmoji(emoji: string) {
  localStorage.setItem(FAV_KEY, JSON.stringify(loadFavoriteEmojis().filter(e => e !== emoji)));
}

/** Kategoriler + arama sözcüğüyle eşleşen emoji listesi döndürür. */
export function searchEmojis(query: string): EmojiEntry[] {
  const q = query.trim().toLowerCase();
  if (!q) return [];
  const out: EmojiEntry[] = [];
  for (const cat of EMOJI_CATEGORIES) {
    for (const entry of cat.emojis) {
      if (entry.k.toLowerCase().includes(q) || (q.length <= 2 && entry.e.includes(q))) {
        out.push(entry);
        if (out.length >= 60) return out;
      }
    }
  }
  return out;
}
