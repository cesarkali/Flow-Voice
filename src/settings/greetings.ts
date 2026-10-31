// Casual, time-aware greeting for the top of Início (Claude-desktop style).
// Every line is gender-neutral on purpose (no "bem-vindo/a", "cansado/a", "madrugador/a",
// "pronto/a", "listo/a"…). Each entry is [with name, without name]; "{n}" is the name.
// One greeting is picked per app open (module scope), avoiding the last one shown.
import type { UiLanguage } from "../lib/types";

type Pair = [string, string];
type Bucket = "late" | "morning" | "afternoon" | "evening" | "any" | "mon" | "fri" | "weekend";

const G: Record<UiLanguage, Record<Bucket, Pair[]>> = {
  pt: {
    late: [
      ["Madrugando, {n}?", "Madrugando por aqui?"],
      ["Insônia criativa, {n}?", "Insônia criativa?"],
      ["O mundo dorme, {n}. A gente escreve.", "O mundo dorme. A gente escreve."],
      ["Boa madrugada, {n}", "Boa madrugada"],
      ["Silêncio bom pra ditar, {n}", "Silêncio bom pra ditar"],
    ],
    morning: [
      ["Bom dia, {n}", "Bom dia"],
      ["Café na mão, {n}?", "Café na mão?"],
      ["Bora começar o dia, {n}?", "Bora começar o dia?"],
      ["Manhã nova, ideias novas, {n}", "Manhã nova, ideias novas"],
      ["Bom dia, {n}. O que sai primeiro hoje?", "Bom dia. O que sai primeiro hoje?"],
      ["Dia começando, {n}. Bora ditar?", "Dia começando. Bora ditar?"],
    ],
    afternoon: [
      ["Boa tarde, {n}", "Boa tarde"],
      ["Tarde produtiva, {n}?", "Tarde produtiva?"],
      ["E essa tarde, {n}? Bora escrever?", "E essa tarde? Bora escrever?"],
      ["Boa tarde, {n}. Fala que eu escrevo.", "Boa tarde. Fala que eu escrevo."],
      ["Pós-almoço com tudo, {n}?", "Pós-almoço com tudo?"],
    ],
    evening: [
      ["Boa noite, {n}", "Boa noite"],
      ["Noite tranquila, {n}?", "Noite tranquila?"],
      ["Fechando o dia, {n}?", "Fechando o dia?"],
      ["Boa noite, {n}. Sem pressa, é só falar.", "Boa noite. Sem pressa, é só falar."],
      ["Última mensagem do dia, {n}?", "Última mensagem do dia?"],
      ["Hora de colocar as ideias em ordem, {n}", "Hora de colocar as ideias em ordem"],
    ],
    any: [
      ["E aí, {n}, o que vamos escrever hoje?", "E aí, o que vamos escrever hoje?"],
      ["Bora ditar, {n}?", "Bora ditar?"],
      ["Fala que eu escrevo, {n}", "Fala que eu escrevo"],
      ["Que bom te ver, {n}", "Que bom te ver por aqui"],
      ["Olá de novo, {n}", "Olá de novo"],
      ["Tudo certo por aí, {n}?", "Tudo certo por aí?"],
      ["Sua voz, seu texto, {n}", "Sua voz, seu texto"],
      ["Menos digitação, mais ideia, {n}", "Menos digitação, mais ideia"],
    ],
    mon: [
      ["Segunda de novo, {n}. Bora?", "Segunda de novo. Bora?"],
      ["Semana nova, {n}. Por onde começamos?", "Semana nova. Por onde começamos?"],
      ["Boa segunda, {n}", "Boa segunda"],
    ],
    fri: [
      ["Sextou, {n}!", "Sextou!"],
      ["Sexta-feira, {n}. Só mais umas mensagens", "Sexta-feira. Só mais umas mensagens"],
      ["Boa sexta, {n}", "Boa sexta"],
    ],
    weekend: [
      ["Fim de semana e você por aqui, {n}?", "Fim de semana e você por aqui?"],
      ["Bom fim de semana, {n}", "Bom fim de semana"],
      ["Projeto de fim de semana, {n}?", "Projeto de fim de semana?"],
    ],
  },
  en: {
    late: [
      ["Up late, {n}?", "Up late?"],
      ["Burning the midnight oil, {n}?", "Burning the midnight oil?"],
      ["The world's asleep, {n}. Let's write.", "The world's asleep. Let's write."],
      ["Quiet hours, {n}. Perfect for dictating", "Quiet hours. Perfect for dictating"],
      ["Night owl mode, {n}?", "Night owl mode?"],
    ],
    morning: [
      ["Good morning, {n}", "Good morning"],
      ["Coffee in hand, {n}?", "Coffee in hand?"],
      ["Morning, {n}. What's first today?", "Morning. What's first today?"],
      ["Fresh morning, fresh ideas, {n}", "Fresh morning, fresh ideas"],
      ["Let's get the day going, {n}", "Let's get the day going"],
      ["Rise and dictate, {n}", "Rise and dictate"],
    ],
    afternoon: [
      ["Good afternoon, {n}", "Good afternoon"],
      ["Productive afternoon, {n}?", "Productive afternoon?"],
      ["Afternoon, {n}. You talk, I type.", "Afternoon. You talk, I type."],
      ["Post-lunch push, {n}?", "Post-lunch push?"],
      ["How's the afternoon, {n}?", "How's the afternoon going?"],
    ],
    evening: [
      ["Good evening, {n}", "Good evening"],
      ["Evening, {n}. No rush, just talk.", "Evening. No rush, just talk."],
      ["Wrapping up the day, {n}?", "Wrapping up the day?"],
      ["Last message of the day, {n}?", "Last message of the day?"],
      ["Quiet evening, {n}?", "Quiet evening?"],
      ["Time to put the thoughts in order, {n}", "Time to put the thoughts in order"],
    ],
    any: [
      ["Hey {n}, what are we writing today?", "Hey, what are we writing today?"],
      ["Ready when you are, {n}", "Ready when you are"],
      ["You talk, I type, {n}", "You talk, I type"],
      ["Good to see you, {n}", "Good to see you"],
      ["Hello again, {n}", "Hello again"],
      ["All good, {n}?", "All good over there?"],
      ["Your voice, your words, {n}", "Your voice, your words"],
      ["Less typing, more thinking, {n}", "Less typing, more thinking"],
    ],
    mon: [
      ["Monday again, {n}. Shall we?", "Monday again. Shall we?"],
      ["New week, {n}. Where do we start?", "New week. Where do we start?"],
      ["Happy Monday, {n}", "Happy Monday"],
    ],
    fri: [
      ["Happy Friday, {n}!", "Happy Friday!"],
      ["Friday, {n}. Just a few more messages", "Friday. Just a few more messages"],
      ["Finally Friday, {n}", "Finally Friday"],
    ],
    weekend: [
      ["Weekend and you're here, {n}?", "Weekend and you're here?"],
      ["Happy weekend, {n}", "Happy weekend"],
      ["Weekend project, {n}?", "Weekend project?"],
    ],
  },
  es: {
    late: [
      ["¿Trasnochando, {n}?", "¿Trasnochando?"],
      ["¿Insomnio creativo, {n}?", "¿Insomnio creativo?"],
      ["El mundo duerme, {n}. Nosotros escribimos.", "El mundo duerme. A escribir."],
      ["Buena madrugada, {n}", "Buena madrugada"],
      ["Silencio perfecto para dictar, {n}", "Silencio perfecto para dictar"],
    ],
    morning: [
      ["Buenos días, {n}", "Buenos días"],
      ["¿Café en mano, {n}?", "¿Café en mano?"],
      ["¿Empezamos el día, {n}?", "¿Empezamos el día?"],
      ["Mañana nueva, ideas nuevas, {n}", "Mañana nueva, ideas nuevas"],
      ["Buenos días, {n}. ¿Qué sale primero hoy?", "Buenos días. ¿Qué sale primero hoy?"],
      ["¿Madrugando, {n}?", "¿Madrugando?"],
    ],
    afternoon: [
      ["Buenas tardes, {n}", "Buenas tardes"],
      ["¿Tarde productiva, {n}?", "¿Tarde productiva?"],
      ["Buenas tardes, {n}. Tú hablas, yo escribo.", "Buenas tardes. Tú hablas, yo escribo."],
      ["¿Cómo va la tarde, {n}?", "¿Cómo va la tarde?"],
      ["¿Sobremesa productiva, {n}?", "¿Sobremesa productiva?"],
    ],
    evening: [
      ["Buenas noches, {n}", "Buenas noches"],
      ["¿Noche tranquila, {n}?", "¿Noche tranquila?"],
      ["¿Cerrando el día, {n}?", "¿Cerrando el día?"],
      ["Buenas noches, {n}. Sin prisa, solo habla.", "Buenas noches. Sin prisa, solo habla."],
      ["¿Último mensaje del día, {n}?", "¿Último mensaje del día?"],
      ["Hora de ordenar las ideas, {n}", "Hora de ordenar las ideas"],
    ],
    any: [
      ["Hola, {n}, ¿qué escribimos hoy?", "Hola, ¿qué escribimos hoy?"],
      ["¿Dictamos, {n}?", "¿Dictamos algo?"],
      ["Tú hablas, yo escribo, {n}", "Tú hablas, yo escribo"],
      ["Qué gusto verte, {n}", "Qué gusto verte por aquí"],
      ["Hola de nuevo, {n}", "Hola de nuevo"],
      ["¿Todo bien por ahí, {n}?", "¿Todo bien por ahí?"],
      ["Tu voz, tu texto, {n}", "Tu voz, tu texto"],
      ["Menos teclear, más pensar, {n}", "Menos teclear, más pensar"],
    ],
    mon: [
      ["Lunes otra vez, {n}. ¿Vamos?", "Lunes otra vez. ¿Vamos?"],
      ["Semana nueva, {n}. ¿Por dónde empezamos?", "Semana nueva. ¿Por dónde empezamos?"],
      ["Feliz lunes, {n}", "Feliz lunes"],
    ],
    fri: [
      ["¡Por fin viernes, {n}!", "¡Por fin viernes!"],
      ["Viernes, {n}. Solo unos mensajes más", "Viernes. Solo unos mensajes más"],
      ["Feliz viernes, {n}", "Feliz viernes"],
    ],
    weekend: [
      ["¿Fin de semana y tú por aquí, {n}?", "¿Fin de semana y tú por aquí?"],
      ["Buen fin de semana, {n}", "Buen fin de semana"],
      ["¿Proyecto de fin de semana, {n}?", "¿Proyecto de fin de semana?"],
    ],
  },
};

/** Total greeting lines per language (for reference/tests). */
export const GREETING_COUNT = Object.values(G.pt).reduce((a, b) => a + b.length, 0);

function pool(lang: UiLanguage, d: Date): Pair[] {
  const g = G[lang] ?? G.pt;
  const h = d.getHours();
  const time: Bucket = h < 5 ? "late" : h < 12 ? "morning" : h < 18 ? "afternoon" : "evening";
  const wd = d.getDay();
  const day: Pair[] = wd === 1 ? g.mon : wd === 5 ? g.fri : wd === 0 || wd === 6 ? g.weekend : [];
  // time-of-day lines weigh the most; weekday lines show up often on their day
  return [...g[time], ...g[time], ...g.any, ...day, ...day];
}

const KEY = "fv.greet.last";
/** Picked once per app open: stable while the window lives, re-rolled on the next launch. */
const SEED = Math.random();
let picked: { lang: string; hour: string; named: boolean; text: string } | null = null;

/**
 * Greeting template for now. "{n}" marks where the name goes (only when `named`).
 * Stable for this app session (per language/time bucket/name presence).
 */
export function greeting(lang: UiLanguage, named: boolean, now = new Date()): string {
  const p = pool(lang, now);
  const hour = `${now.toDateString()}:${p.length}:${now.getHours() < 5 ? 0 : now.getHours() < 12 ? 1 : now.getHours() < 18 ? 2 : 3}`;
  if (picked && picked.lang === lang && picked.hour === hour && picked.named === named) return picked.text;
  let last = "";
  try {
    last = localStorage.getItem(KEY) ?? "";
  } catch {}
  let i = Math.floor(SEED * p.length);
  const txt = (k: number) => p[k % p.length][named ? 0 : 1];
  // don't greet the same way twice in a row
  for (let n = 0; n < p.length && txt(i) === last; n++) i++;
  const text = txt(i);
  picked = { lang, hour, named, text };
  try {
    localStorage.setItem(KEY, text);
  } catch {}
  return text;
}
