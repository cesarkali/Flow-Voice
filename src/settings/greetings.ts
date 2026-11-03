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
  // fr/de/ja/zh/ru/hi/bn: same rule, no gendered adjective or verb about the person
  // (fr: no "prêt·e", "matinal·e"; ru: no past tense or short adjectives; hi: no -ा/-ी verb endings).
  fr: {
    late: [
      ["Encore debout, {n} ?", "Encore debout ?"],
      ["Insomnie créative, {n} ?", "Insomnie créative ?"],
      ["Le monde dort, {n}. On écrit.", "Le monde dort. On écrit."],
      ["Le calme de la nuit, {n}. Parfait pour dicter", "Le calme de la nuit. Parfait pour dicter"],
      ["Mode oiseau de nuit, {n} ?", "Mode oiseau de nuit ?"],
    ],
    morning: [
      ["Bonjour, {n}", "Bonjour"],
      ["Café à la main, {n} ?", "Café à la main ?"],
      ["Bonjour, {n}. On commence par quoi ?", "Bonjour. On commence par quoi ?"],
      ["Nouveau matin, nouvelles idées, {n}", "Nouveau matin, nouvelles idées"],
      ["On lance la journée, {n} ?", "On lance la journée ?"],
    ],
    afternoon: [
      ["Bon après-midi, {n}", "Bon après-midi"],
      ["Après-midi productif, {n} ?", "Après-midi productif ?"],
      ["Bon après-midi, {n}. Parlez, j'écris.", "Bon après-midi. Parlez, j'écris."],
      ["Comment va l'après-midi, {n} ?", "Comment va l'après-midi ?"],
      ["Coup de boost après le déjeuner, {n} ?", "Coup de boost après le déjeuner ?"],
    ],
    evening: [
      ["Bonsoir, {n}", "Bonsoir"],
      ["Soirée tranquille, {n} ?", "Soirée tranquille ?"],
      ["On boucle la journée, {n} ?", "On boucle la journée ?"],
      ["Bonsoir, {n}. Sans hâte, il suffit de parler.", "Bonsoir. Sans hâte, il suffit de parler."],
      ["Dernier message de la journée, {n} ?", "Dernier message de la journée ?"],
    ],
    any: [
      ["Salut {n}, on écrit quoi aujourd'hui ?", "Salut, on écrit quoi aujourd'hui ?"],
      ["Parlez, j'écris, {n}", "Parlez, j'écris"],
      ["Bon retour, {n}", "Bon retour parmi nous"],
      ["Re-bonjour, {n}", "Re-bonjour"],
      ["Tout va bien, {n} ?", "Tout va bien de votre côté ?"],
      ["Votre voix, vos mots, {n}", "Votre voix, vos mots"],
      ["Moins de frappe, plus d'idées, {n}", "Moins de frappe, plus d'idées"],
    ],
    mon: [
      ["Encore lundi, {n}. On y va ?", "Encore lundi. On y va ?"],
      ["Nouvelle semaine, {n}. On commence où ?", "Nouvelle semaine. On commence où ?"],
      ["Bon lundi, {n}", "Bon lundi"],
    ],
    fri: [
      ["Enfin vendredi, {n} !", "Enfin vendredi !"],
      ["Vendredi, {n}. Encore quelques messages", "Vendredi. Encore quelques messages"],
      ["Bon vendredi, {n}", "Bon vendredi"],
    ],
    weekend: [
      ["Le week-end, et vous voilà, {n} ?", "Le week-end, et vous voilà ?"],
      ["Bon week-end, {n}", "Bon week-end"],
      ["Projet du week-end, {n} ?", "Projet du week-end ?"],
    ],
  },
  de: {
    late: [
      ["Noch wach, {n}?", "Noch wach?"],
      ["Kreative Schlaflosigkeit, {n}?", "Kreative Schlaflosigkeit?"],
      ["Die Welt schläft, {n}. Wir schreiben.", "Die Welt schläft. Wir schreiben."],
      ["Stille Stunden, {n}. Perfekt zum Diktieren", "Stille Stunden. Perfekt zum Diktieren"],
      ["Nachteulen-Modus, {n}?", "Nachteulen-Modus?"],
    ],
    morning: [
      ["Guten Morgen, {n}", "Guten Morgen"],
      ["Kaffee schon in der Hand, {n}?", "Kaffee schon in der Hand?"],
      ["Morgen, {n}. Was kommt heute zuerst?", "Morgen. Was kommt heute zuerst?"],
      ["Neuer Morgen, neue Ideen, {n}", "Neuer Morgen, neue Ideen"],
      ["Auf in den Tag, {n}?", "Auf in den Tag?"],
    ],
    afternoon: [
      ["Guten Tag, {n}", "Guten Tag"],
      ["Produktiver Nachmittag, {n}?", "Produktiver Nachmittag?"],
      ["Hallo {n}. Du sprichst, ich schreibe.", "Hallo. Du sprichst, ich schreibe."],
      ["Wie läuft der Nachmittag, {n}?", "Wie läuft der Nachmittag?"],
      ["Endspurt nach dem Mittagessen, {n}?", "Endspurt nach dem Mittagessen?"],
    ],
    evening: [
      ["Guten Abend, {n}", "Guten Abend"],
      ["Ruhiger Abend, {n}?", "Ruhiger Abend?"],
      ["Den Tag abschließen, {n}?", "Den Tag abschließen?"],
      ["Guten Abend, {n}. Ganz ohne Eile, einfach sprechen.", "Guten Abend. Ganz ohne Eile, einfach sprechen."],
      ["Letzte Nachricht für heute, {n}?", "Letzte Nachricht für heute?"],
    ],
    any: [
      ["Hey {n}, was schreiben wir heute?", "Hey, was schreiben wir heute?"],
      ["Du sprichst, ich schreibe, {n}", "Du sprichst, ich schreibe"],
      ["Schön, dich zu sehen, {n}", "Schön, dich zu sehen"],
      ["Hallo nochmal, {n}", "Hallo nochmal"],
      ["Alles gut, {n}?", "Alles gut bei dir?"],
      ["Deine Stimme, deine Worte, {n}", "Deine Stimme, deine Worte"],
      ["Weniger tippen, mehr denken, {n}", "Weniger tippen, mehr denken"],
    ],
    mon: [
      ["Wieder Montag, {n}. Legen wir los?", "Wieder Montag. Legen wir los?"],
      ["Neue Woche, {n}. Wo fangen wir an?", "Neue Woche. Wo fangen wir an?"],
      ["Guten Start in die Woche, {n}", "Guten Start in die Woche"],
    ],
    fri: [
      ["Endlich Freitag, {n}!", "Endlich Freitag!"],
      ["Freitag, {n}. Nur noch ein paar Nachrichten", "Freitag. Nur noch ein paar Nachrichten"],
      ["Schönen Freitag, {n}", "Schönen Freitag"],
    ],
    weekend: [
      ["Wochenende, und du bist hier, {n}?", "Wochenende, und du bist hier?"],
      ["Schönes Wochenende, {n}", "Schönes Wochenende"],
      ["Wochenendprojekt, {n}?", "Wochenendprojekt?"],
    ],
  },
  ja: {
    late: [
      ["夜更かしですか、{n}さん？", "夜更かしですか？"],
      ["深夜のひらめきですね、{n}さん", "深夜のひらめきですね"],
      ["世界は眠っています、{n}さん。書きましょう。", "世界は眠っています。書きましょう。"],
      ["静かな時間です、{n}さん。音声入力にぴったり", "静かな時間です。音声入力にぴったり"],
    ],
    morning: [
      ["おはようございます、{n}さん", "おはようございます"],
      ["コーヒーはお手元に、{n}さん？", "コーヒーはお手元に？"],
      ["おはようございます、{n}さん。今日は何から？", "おはようございます。今日は何から？"],
      ["新しい朝、新しいアイデア、{n}さん", "新しい朝、新しいアイデア"],
      ["今日も始めましょう、{n}さん", "今日も始めましょう"],
    ],
    afternoon: [
      ["こんにちは、{n}さん", "こんにちは"],
      ["午後もはかどっていますか、{n}さん？", "午後もはかどっていますか？"],
      ["こんにちは、{n}さん。話してくれれば書きます。", "こんにちは。話してくれれば書きます。"],
      ["午後の調子はいかがですか、{n}さん？", "午後の調子はいかがですか？"],
    ],
    evening: [
      ["こんばんは、{n}さん", "こんばんは"],
      ["穏やかな夜ですね、{n}さん", "穏やかな夜ですね"],
      ["一日の締めくくりですか、{n}さん？", "一日の締めくくりですか？"],
      ["こんばんは、{n}さん。急がず、話すだけで大丈夫。", "こんばんは。急がず、話すだけで大丈夫。"],
      ["今日最後のメッセージですか、{n}さん？", "今日最後のメッセージですか？"],
    ],
    any: [
      ["{n}さん、今日は何を書きましょう？", "今日は何を書きましょう？"],
      ["話してくれれば書きます、{n}さん", "話してくれれば書きます"],
      ["お会いできてうれしいです、{n}さん", "お会いできてうれしいです"],
      ["また会いましたね、{n}さん", "また会いましたね"],
      ["あなたの声を、あなたの言葉に、{n}さん", "あなたの声を、あなたの言葉に"],
      ["打つより、話そう、{n}さん", "打つより、話そう"],
    ],
    mon: [
      ["また月曜日ですね、{n}さん。始めましょうか？", "また月曜日ですね。始めましょうか？"],
      ["新しい一週間です、{n}さん。どこから始めますか？", "新しい一週間です。どこから始めますか？"],
    ],
    fri: [
      ["やっと金曜日ですね、{n}さん！", "やっと金曜日ですね！"],
      ["金曜日です、{n}さん。あと少しのメッセージ", "金曜日です。あと少しのメッセージ"],
    ],
    weekend: [
      ["週末なのにお仕事ですか、{n}さん？", "週末なのにお仕事ですか？"],
      ["良い週末を、{n}さん", "良い週末を"],
      ["週末のプロジェクトですか、{n}さん？", "週末のプロジェクトですか？"],
    ],
  },
  zh: {
    late: [
      ["还没睡呀，{n}？", "还没睡呀？"],
      ["深夜灵感来了，{n}？", "深夜灵感来了？"],
      ["世界在沉睡，{n}。我们来写。", "世界在沉睡。我们来写。"],
      ["夜深人静，{n}，正适合听写", "夜深人静，正适合听写"],
    ],
    morning: [
      ["早上好，{n}", "早上好"],
      ["咖啡在手了吗，{n}？", "咖啡在手了吗？"],
      ["早上好，{n}。今天先写什么？", "早上好。今天先写什么？"],
      ["新的早晨，新的想法，{n}", "新的早晨，新的想法"],
      ["开始新的一天吧，{n}", "开始新的一天吧"],
    ],
    afternoon: [
      ["下午好，{n}", "下午好"],
      ["下午效率如何，{n}？", "下午效率如何？"],
      ["下午好，{n}。你来说，我来写。", "下午好。你来说，我来写。"],
      ["午饭后再冲一把，{n}？", "午饭后再冲一把？"],
    ],
    evening: [
      ["晚上好，{n}", "晚上好"],
      ["安静的夜晚，{n}？", "安静的夜晚？"],
      ["准备收工了吗，{n}？", "准备收工了吗？"],
      ["晚上好，{n}。不着急，说就行。", "晚上好。不着急，说就行。"],
      ["今天最后一条消息，{n}？", "今天最后一条消息？"],
    ],
    any: [
      ["嗨，{n}，今天写点什么？", "嗨，今天写点什么？"],
      ["你来说，我来写，{n}", "你来说，我来写"],
      ["很高兴见到你，{n}", "很高兴见到你"],
      ["又见面了，{n}", "又见面了"],
      ["一切都好吗，{n}？", "一切都好吗？"],
      ["你的声音，你的文字，{n}", "你的声音，你的文字"],
      ["少打字，多思考，{n}", "少打字，多思考"],
    ],
    mon: [
      ["又是星期一，{n}。开工吧？", "又是星期一。开工吧？"],
      ["新的一周，{n}。从哪儿开始？", "新的一周。从哪儿开始？"],
    ],
    fri: [
      ["终于星期五了，{n}！", "终于星期五了！"],
      ["星期五，{n}。再写几条消息就好", "星期五。再写几条消息就好"],
    ],
    weekend: [
      ["周末还在这里，{n}？", "周末还在这里？"],
      ["周末愉快，{n}", "周末愉快"],
      ["周末项目，{n}？", "周末项目？"],
    ],
  },
  ru: {
    late: [
      ["Не спится, {n}?", "Не спится?"],
      ["Творческая бессонница, {n}?", "Творческая бессонница?"],
      ["Мир спит, {n}. А мы пишем.", "Мир спит. А мы пишем."],
      ["Тихие часы, {n}. Самое время диктовать", "Тихие часы. Самое время диктовать"],
    ],
    morning: [
      ["Доброе утро, {n}", "Доброе утро"],
      ["Кофе уже в руке, {n}?", "Кофе уже в руке?"],
      ["Доброе утро, {n}. С чего начнём?", "Доброе утро. С чего начнём?"],
      ["Новое утро, новые идеи, {n}", "Новое утро, новые идеи"],
      ["Начнём день, {n}?", "Начнём день?"],
    ],
    afternoon: [
      ["Добрый день, {n}", "Добрый день"],
      ["Продуктивный день, {n}?", "Продуктивный день?"],
      ["Добрый день, {n}. Говорите, я пишу.", "Добрый день. Говорите, я пишу."],
      ["Как проходит день, {n}?", "Как проходит день?"],
    ],
    evening: [
      ["Добрый вечер, {n}", "Добрый вечер"],
      ["Спокойный вечер, {n}?", "Спокойный вечер?"],
      ["Завершаем день, {n}?", "Завершаем день?"],
      ["Добрый вечер, {n}. Без спешки, просто говорите.", "Добрый вечер. Без спешки, просто говорите."],
      ["Последнее сообщение за день, {n}?", "Последнее сообщение за день?"],
    ],
    any: [
      ["Привет, {n}! Что пишем сегодня?", "Привет! Что пишем сегодня?"],
      ["Говорите, я пишу, {n}", "Говорите, я пишу"],
      ["Рады встрече, {n}", "Рады встрече"],
      ["Снова здравствуйте, {n}", "Снова здравствуйте"],
      ["Всё в порядке, {n}?", "Всё в порядке?"],
      ["Ваш голос, ваши слова, {n}", "Ваш голос, ваши слова"],
      ["Меньше печатать, больше думать, {n}", "Меньше печатать, больше думать"],
    ],
    mon: [
      ["Снова понедельник, {n}. Поехали?", "Снова понедельник. Поехали?"],
      ["Новая неделя, {n}. С чего начнём?", "Новая неделя. С чего начнём?"],
      ["Хорошего понедельника, {n}", "Хорошего понедельника"],
    ],
    fri: [
      ["Наконец-то пятница, {n}!", "Наконец-то пятница!"],
      ["Пятница, {n}. Ещё пара сообщений", "Пятница. Ещё пара сообщений"],
      ["Хорошей пятницы, {n}", "Хорошей пятницы"],
    ],
    weekend: [
      ["Выходные, а вы здесь, {n}?", "Выходные, а вы здесь?"],
      ["Хороших выходных, {n}", "Хороших выходных"],
      ["Проект на выходные, {n}?", "Проект на выходные?"],
    ],
  },
  hi: {
    late: [
      ["देर रात तक काम, {n}?", "देर रात तक काम?"],
      ["रात का रचनात्मक समय, {n}?", "रात का रचनात्मक समय?"],
      ["दुनिया सो रही है, {n}। चलिए लिखें।", "दुनिया सो रही है। चलिए लिखें।"],
      ["शांत घड़ी, {n}। डिक्टेशन के लिए बढ़िया", "शांत घड़ी। डिक्टेशन के लिए बढ़िया"],
    ],
    morning: [
      ["सुप्रभात, {n}", "सुप्रभात"],
      ["हाथ में चाय या कॉफ़ी, {n}?", "हाथ में चाय या कॉफ़ी?"],
      ["सुप्रभात, {n}। आज शुरुआत किससे?", "सुप्रभात। आज शुरुआत किससे?"],
      ["नई सुबह, नए विचार, {n}", "नई सुबह, नए विचार"],
      ["चलिए दिन शुरू करें, {n}", "चलिए दिन शुरू करें"],
    ],
    afternoon: [
      ["नमस्कार, {n}", "नमस्कार"],
      ["दोपहर का काम कैसा चल रहा है, {n}?", "दोपहर का काम कैसा चल रहा है?"],
      ["नमस्कार, {n}। बोलिए, लिखने का काम मेरा।", "नमस्कार। बोलिए, लिखने का काम मेरा।"],
      ["खाने के बाद थोड़ा और काम, {n}?", "खाने के बाद थोड़ा और काम?"],
    ],
    evening: [
      ["शुभ संध्या, {n}", "शुभ संध्या"],
      ["शांत शाम, {n}?", "शांत शाम?"],
      ["दिन समेटने का समय, {n}?", "दिन समेटने का समय?"],
      ["शुभ संध्या, {n}। कोई जल्दी नहीं, बस बोलिए।", "शुभ संध्या। कोई जल्दी नहीं, बस बोलिए।"],
      ["आज का आख़िरी संदेश, {n}?", "आज का आख़िरी संदेश?"],
    ],
    any: [
      ["नमस्ते {n}, आज क्या लिखें?", "नमस्ते, आज क्या लिखें?"],
      ["बोलिए, लिखने का काम मेरा, {n}", "बोलिए, लिखने का काम मेरा"],
      ["फिर से नमस्ते, {n}", "फिर से नमस्ते"],
      ["सब ठीक, {n}?", "सब ठीक?"],
      ["आपकी आवाज़, आपके शब्द, {n}", "आपकी आवाज़, आपके शब्द"],
      ["कम टाइपिंग, ज़्यादा सोच, {n}", "कम टाइपिंग, ज़्यादा सोच"],
    ],
    mon: [
      ["फिर से सोमवार, {n}। शुरू करें?", "फिर से सोमवार। शुरू करें?"],
      ["नया हफ़्ता, {n}। कहां से शुरुआत?", "नया हफ़्ता। कहां से शुरुआत?"],
    ],
    fri: [
      ["आख़िरकार शुक्रवार, {n}!", "आख़िरकार शुक्रवार!"],
      ["शुक्रवार, {n}। बस कुछ संदेश और", "शुक्रवार। बस कुछ संदेश और"],
    ],
    weekend: [
      ["सप्ताहांत में भी यहां, {n}?", "सप्ताहांत में भी यहां?"],
      ["सप्ताहांत शुभ हो, {n}", "सप्ताहांत शुभ हो"],
      ["सप्ताहांत का प्रोजेक्ट, {n}?", "सप्ताहांत का प्रोजेक्ट?"],
    ],
  },
  bn: {
    late: [
      ["এখনও জেগে, {n}?", "এখনও জেগে?"],
      ["সৃজনশীল নির্ঘুম রাত, {n}?", "সৃজনশীল নির্ঘুম রাত?"],
      ["পৃথিবী ঘুমিয়ে, {n}। চলুন লিখি।", "পৃথিবী ঘুমিয়ে। চলুন লিখি।"],
      ["নিস্তব্ধ সময়, {n}। ডিকটেশনের জন্য দারুণ", "নিস্তব্ধ সময়। ডিকটেশনের জন্য দারুণ"],
    ],
    morning: [
      ["সুপ্রভাত, {n}", "সুপ্রভাত"],
      ["হাতে চা না কফি, {n}?", "হাতে চা না কফি?"],
      ["সুপ্রভাত, {n}। আজ শুরু কী দিয়ে?", "সুপ্রভাত। আজ শুরু কী দিয়ে?"],
      ["নতুন সকাল, নতুন ভাবনা, {n}", "নতুন সকাল, নতুন ভাবনা"],
      ["চলুন দিন শুরু করি, {n}", "চলুন দিন শুরু করি"],
    ],
    afternoon: [
      ["শুভ অপরাহ্ন, {n}", "শুভ অপরাহ্ন"],
      ["দুপুরটা কাজে কাটছে, {n}?", "দুপুরটা কাজে কাটছে?"],
      ["শুভ অপরাহ্ন, {n}। আপনি বলুন, আমি লিখি।", "শুভ অপরাহ্ন। আপনি বলুন, আমি লিখি।"],
      ["দুপুরের খাবারের পর আরেকটু কাজ, {n}?", "দুপুরের খাবারের পর আরেকটু কাজ?"],
    ],
    evening: [
      ["শুভ সন্ধ্যা, {n}", "শুভ সন্ধ্যা"],
      ["শান্ত সন্ধ্যা, {n}?", "শান্ত সন্ধ্যা?"],
      ["দিন গুটিয়ে আনছেন, {n}?", "দিন গুটিয়ে আনছেন?"],
      ["শুভ সন্ধ্যা, {n}। তাড়া নেই, শুধু বলুন।", "শুভ সন্ধ্যা। তাড়া নেই, শুধু বলুন।"],
      ["আজকের শেষ বার্তা, {n}?", "আজকের শেষ বার্তা?"],
    ],
    any: [
      ["হ্যালো {n}, আজ কী লিখব?", "হ্যালো, আজ কী লিখব?"],
      ["আপনি বলুন, আমি লিখি, {n}", "আপনি বলুন, আমি লিখি"],
      ["আপনাকে দেখে ভালো লাগল, {n}", "আপনাকে দেখে ভালো লাগল"],
      ["আবার দেখা হলো, {n}", "আবার দেখা হলো"],
      ["সব ঠিক আছে তো, {n}?", "সব ঠিক আছে তো?"],
      ["আপনার কণ্ঠ, আপনার শব্দ, {n}", "আপনার কণ্ঠ, আপনার শব্দ"],
      ["কম টাইপ, বেশি ভাবনা, {n}", "কম টাইপ, বেশি ভাবনা"],
    ],
    mon: [
      ["আবার সোমবার, {n}। শুরু করি?", "আবার সোমবার। শুরু করি?"],
      ["নতুন সপ্তাহ, {n}। কোথা থেকে শুরু?", "নতুন সপ্তাহ। কোথা থেকে শুরু?"],
    ],
    fri: [
      ["অবশেষে শুক্রবার, {n}!", "অবশেষে শুক্রবার!"],
      ["শুক্রবার, {n}। আর মাত্র কয়েকটা বার্তা", "শুক্রবার। আর মাত্র কয়েকটা বার্তা"],
    ],
    weekend: [
      ["সপ্তাহান্তেও এখানে, {n}?", "সপ্তাহান্তেও এখানে?"],
      ["শুভ সপ্তাহান্ত, {n}", "শুভ সপ্তাহান্ত"],
      ["সপ্তাহান্তের প্রজেক্ট, {n}?", "সপ্তাহান্তের প্রজেক্ট?"],
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
