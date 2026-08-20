<!-- source: README.md sha256:1f5bf984e975 -->
# Codewhale

Un agent de programmation open source pour votre terminal — apportez votre propre modèle.

Codewhale a commencé comme une expérience native pour DeepSeek. Il est depuis
devenu un projet porté par la communauté : un harness de programmation qui
convient à une communauté internationale en croissance et prend en charge autant
de modèles et de fournisseurs que possible — les modèles ouverts d'abord,
hébergés ou locaux, sans en privilégier aucun.

Donnez-lui un fournisseur, un modèle et une tâche. Il lit votre code, modifie
des fichiers, exécute des commandes et vérifie son propre travail, puis
s'arrête quand la tâche est terminée ou qu'il a besoin de vous. Changez de
modèle en cours de tâche avec `/model`. Travaillez de façon interactive dans
la TUI, ou lancez `codewhale exec` dans des scripts et en CI. Écrit en Rust,
sous licence MIT, il tourne sur votre machine.

Ce que les autres harness n'ont pas : **vous choisissez le modèle de chaque
rôle — et rien ne les oblige à correspondre** — et **les agents de Codewhale
se parlent, à travers les modèles.** Une fleet fixe un fournisseur, un
modèle et un niveau de raisonnement par rôle, si bien qu'un modèle rapide et
bon marché peut en diriger un autre coûteux et raisonneur, ou qu'un builder
GLM peut travailler sur la même tâche qu'un reviewer Kimi. Pendant qu'ils
tournent, envoyez une note à n'importe lequel en plein vol, lisez son
transcript ou interrompez-le — et ce n'est pas limité aux liens
parent-enfant : des tâches Codewhale distinctes d'un même workspace
s'échangent de l'Agent Mail durable qui survit aux redémarrages, livrée
exactement une fois à une frontière sûre, avec les identifiants masqués. Un
`/goal` tient un objectif long sur plusieurs tours, jusqu'à ce qu'il soit
réellement fini. Les rôles sont des fichiers que vous éditez, et tout le
harness reste le vôtre.

Nous cherchons toujours des contributeurs et des façons de nous améliorer. Si un
modèle ou un fournisseur que vous utilisez manque, ou si quelque chose casse,
nous le dire est l'une des choses les plus utiles que vous puissiez faire —
voir [Contribuer](#contribuer).

[English](README.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja-JP.md) · [Tiếng Việt](README.vi.md) · [Bahasa Indonesia](README.id.md) · [한국어](README.ko-KR.md) · [Español](README.es-419.md) · [Português](README.pt-BR.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · [Deutsch](README.de.md) · [繁體中文](README.zh-TW.md) · [हिन्दी](README.hi.md) · [Türkçe](README.tr.md) · [Italiano](README.it.md) · [Polski](README.pl.md) · [العربية](README.ar.md) · [Català](README.ca.md) · [codewhale.net](https://codewhale.net/) · [Docs](docs) · [Changelog](CHANGELOG.md) · [Discord](https://discord.gg/37gfS3ksug)

[![CI](https://github.com/Hmbown/CodeWhale/actions/workflows/ci.yml/badge.svg)](https://github.com/Hmbown/CodeWhale/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/codewhale-cli?label=crates.io)](https://crates.io/crates/codewhale-cli)
[![npm](https://img.shields.io/npm/v/codewhale?label=npm)](https://www.npmjs.com/package/codewhale)
[![Discord](https://img.shields.io/badge/Discord-join%20the%20community-5865F2?logo=discord&logoColor=white)](https://discord.gg/37gfS3ksug)

![Codewhale en cours d'exécution dans un terminal](assets/screenshot.webp)

## Installation

```bash
npm install -g codewhale
```

Cargo, Docker, Nix, Scoop, les archives précompilées, Android/Termux et un
miroir CNB pour ceux qui n'atteignent pas GitHub sont documentés dans
[docs/INSTALL.md](docs/INSTALL.md). Vous venez de `deepseek-tui` ? Votre
configuration et vos sessions sont reprises — voir
[docs/REBRAND.md](docs/REBRAND.md).

## Utilisation

```bash
codewhale auth set --provider deepseek   # or export ANTHROPIC_API_KEY, etc.
codewhale                                # open the TUI
codewhale exec "fix the failing test"    # headless
codewhale web                            # local browser client on 127.0.0.1
```

Dans la TUI : `/model` change fournisseur et modèle ensemble, `/fleet`
constitue et lance l'équipe — un rôle à la fois, chacun avec son propre
modèle —, `/undo` annule le dernier tour, et `/restore <N>` ramène
l'espace de travail à un instantané antérieur (`/restore` seul les
liste). `Tab` fait défiler Plan / Work / Operate quand le compositeur est vide —
s'il contient du texte, `Tab` complète les commandes slash et les mentions
`@`. `Shift+Tab` fait défiler à tout moment la posture de permission Ask /
Auto-Review / Full Access. `!` exécute une commande shell par le chemin
d'approbation habituel.

## Ce qu'il fait

- **N'importe quel modèle, n'importe quel fournisseur — et n'importe quel
  mélange.** DeepSeek, Claude, GPT, Kimi, GLM et plus de 30 fournisseurs,
  plus votre propre vLLM, SGLang ou Ollama sans clé, le tout via un seul
  runtime et un seul jeu d'outils. Le catalogue suit la gamme en direct de
  chaque fournisseur — le backend V4 Pro de DeepSeek (libellé
  `DeepSeek-V4-Pro-0813`) reste appelable en `deepseek-v4-pro`, Grok 4.6 est
  le défaut xAI direct, et OrcaRouter route via `orcarouter/auto`. Un rôle
  enregistré consigne explicitement son `provider`, son `model` et son
  niveau de raisonnement, donc une fleet peut traverser plusieurs éditeurs
  dans une même exécution, et la route d'un rôle ne dépend jamais du
  fournisseur qui se trouve actif. Limites de contexte et prix viennent de
  la vraie route ; un prix inconnu s'affiche comme inconnu, pas comme 0 $.
- **Des agents qui se parlent — à travers les modèles.** Chaque agent de
  Codewhale reste joignable pendant qu'il travaille : `message` met une note
  en file pour un sous-agent en cours, `followup` le réveille avec votre
  note à sa prochaine frontière sûre, `peek` lit son transcript, et une
  interruption n'arrête que son tour. Cela va au-delà de l'arbre
  parent-enfant : des tâches distinctes du même workspace s'échangent de
  l'**Agent Mail** durable — un résumé de passation en file qui survit au
  redémarrage, livré exactement une fois à la frontière sûre du
  destinataire, et qui masque identifiants et chemins. Une session GLM et
  une session Kimi peuvent ainsi se coordonner dans deux terminaux, sans que
  vous serviez de courroie de transmission. Chaque côté peut être un modèle
  différent ; Codewhale porte la conversation.
- **Un harness que vous rédigez.** Les rôles sont des fichiers que vous
  pouvez lire et modifier — un modèle, une posture d'outils et des
  consignes permanentes par rôle — gardés dans le projet pour que l'équipe
  les partage, ou à côté de vos autres réglages personnels pour qu'ils
  vous suivent d'un dépôt à l'autre. Une constitution consigne comment
  vous voulez que l'agent se comporte d'une session à l'autre, pour que le
  harness suive votre pratique plutôt que la nôtre.
- **Lecture seule jusqu'à ce que vous autorisiez davantage.** Le mode Plan
  ne peut pas modifier de fichiers, et les approbations filtrent les
  commandes risquées. Quand un sandbox OS enveloppe réellement une
  commande, Codewhale le dit : Seatbelt sur macOS lorsqu'il est
  disponible, bubblewrap en option sur Linux. Le `constitution.json` d'un
  dépôt se compile en verrous d'écriture que même Full Access ne peut
  contourner.
- **Un travail que vous pouvez reprendre.** Une fleet enregistre chaque
  étape dans un registre en append-only, pour que `fleet resume` reprenne là
  où vous vous êtes arrêté. `/goal` tient un objectif persistant que l'agent
  poursuit de tour en tour — pausable, reprisable, restauré avec la session
  au redémarrage — et `/workflows` ouvre un tableau de bord en direct sur
  toutes les exécutions que conserve le journal de ce workspace.

## Intégrations

- **DeepSeek Harness (dsh) — connecté via Codewhale.**
  `codewhale integrations dsh connect` relie une installation existante de
  `@deepseek-ai/dsh` à votre route de fournisseur Codewhale, vos
  permissions et votre espace de travail, et `integrations dsh
  install-bundle` ajoute le bundle de plugins DSH optionnel pour que
  `dsh --profile codewhale` porte cette identité tout seul. Codewhale
  détient les permissions et l'autorité de cycle de vie ; dsh garde ses
  propres sessions, profils et identifiants intacts. Voir
  [docs/INTEGRATIONS_DSH.md](docs/INTEGRATIONS_DSH.md).
- **VS Code.** Le scaffold officiel de l'extension (`extensions/vscode`)
  ouvre Codewhale dans un terminal intégré et expose une Agent View en
  lecture seule sur le runtime local. C'est une préversion de
  développement local, pas encore une publication marketplace.

## En savoir plus

- [docs/PROVIDERS.md](docs/PROVIDERS.md) — chaque route de fournisseur :
  hébergée, passerelle et locale
- [docs/FLEET.md](docs/FLEET.md) — les fleets, le registre et la reprise
- [docs/WORKFLOW_EXPERIMENTAL_SEARCH.md](docs/WORKFLOW_EXPERIMENTAL_SEARCH.md) — recherche expérimentale figée et neutre vis-à-vis des fournisseurs dans Workflow
- [docs/CONFIGURATION.md](docs/CONFIGURATION.md) — `config.toml`, les
  hooks et la constitution
- [docs/AUTHORIZATION_ORDER.md](docs/AUTHORIZATION_ORDER.md) — comment
  modes, hooks, règles de permission, planchers de sécurité, loi du
  dépôt, approbations et sandbox se composent
- [docs/HOOKS.md](docs/HOOKS.md) — les onze événements de hook du cycle
  de vie TUI, leurs payloads, et lesquels parmi trois peuvent orienter un
  tour (`codewhale exec` et les sous-commandes CLI ne déclenchent pas de
  hooks)
- [docs/WEB.md](docs/WEB.md) — le client navigateur en loopback uniquement
  et sa frontière d'authentification à usage unique

Tout le reste — modes, raccourcis, détails du sandbox, MCP, l'API runtime
et l'architecture — vit dans [docs](docs) et sur
[codewhale.net](https://codewhale.net/).

## Contribuer

Issues, PR, étapes de reproduction, journaux et demandes de
fonctionnalités sont tous du vrai travail de projet, et les premières
contributions sont les bienvenues. Quand une PR ne peut pas être fusionnée
telle quelle, les mainteneurs récoltent ce qui fonctionne et gardent
l'auteur crédité — dans le commit, le changelog et
[docs/CONTRIBUTORS.md](docs/CONTRIBUTORS.md).

- [Issues ouvertes](https://github.com/Hmbown/CodeWhale/issues) — les
  bonnes premières contributions sont ici
- [CONTRIBUTING.md](CONTRIBUTING.md) — installation de dev et flux de PR
- [docs/CONTRIBUTORS.md](docs/CONTRIBUTORS.md) — toutes celles et ceux
  qui ont façonné le projet
- [Offrez-moi un café](https://www.buymeacoffee.com/hmbown)

Merci à [DeepSeek](https://github.com/deepseek-ai) pour les modèles et le
soutien qui ont lancé le projet, à
[DataWhale](https://github.com/datawhalechina) 🐋 de nous avoir accueillis
dans la famille Whale Brother, et à
[OpenWarp](https://github.com/zerx-lab/warp) et
[Open Design](https://github.com/nexu-io/open-design) pour la
collaboration sur l'expérience d'agent dans le terminal.

## Licence

[MIT](LICENSE). Projet communautaire indépendant, non affilié à aucun
fournisseur de modèles.

![Codewhale déployant trois sous-agents scout en lecture seule dans un terminal](assets/fanout.gif)

[![Star History Chart](https://star-history.dera.page/svg?repos=Hmbown/CodeWhale&type=date&legend=top-left)](https://star-history.dera.page/#Hmbown/CodeWhale&type=date)
