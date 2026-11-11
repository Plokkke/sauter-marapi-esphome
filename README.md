# sauter-marapi-esphome

**Français** · [English](README.en.md)

Pilotez votre sèche-serviettes Sauter Marapi depuis Home Assistant, en gardant son écran et ses touches : une petite carte Wi-Fi se glisse sur le bus interne du radiateur.

![Boîtier de commande du Marapi, ouvert](knowledge/photos/command-board-in-box.jpg)

> [!NOTE]
> **État du projet.** Le protocole du bus interne est décodé, et la carte MITM marche sur le radiateur (plaque d'essai, Wemos D1 mini, firmware ESPHome) : les touches et Home Assistant pilotent la carte de puissance. Restent la température, l'alimentation par la nappe, le circuit imprimé et le boîtier.

## Ce que ça permet

- **Suivre le radiateur** : consigne, Boost et arrêt réglés sur les touches, chauffe en cours.
- **Le régler depuis Home Assistant** : consigne, Boost (soufflerie), arrêt.
- **Garder l'appareil d'origine** : l'écran et les touches continuent de marcher comme avant ; le dernier qui parle, touche ou Home Assistant, a raison.
- **Rien à brancher sur le 230 V** : la carte s'alimente sur le 5 V de la nappe, comme le boîtier de commande.
- **Sans risque** : sans Home Assistant, les touches pilotent toujours le radiateur, et un Boost lancé par Home Assistant s'arrête de lui-même. Si la carte elle-même tombe en panne, le radiateur s'arrête (« Err 10 ») : il suffit de la débrancher et de reconnecter la nappe d'origine.

## Compatibilité

Étudié sur un **Sauter Marapi tactile 1750 W** (750 W de fluide et 1000 W de soufflerie), boîtier de commande `HM011 TD1N01 RAD`. Les sèche-serviettes et radiateurs du groupe Atlantic (Atlantic, Thermor, Sauter) qui affichent « Err 10 » quand leur nappe interne est débranchée utilisent probablement le même bus : à vérifier avec l'écoute de `discovery/`.

## Comment ça marche

Le radiateur a deux cartes, reliées par une nappe de trois fils (masse, 5 V, données) :

- le **boîtier de commande** (écran, touches tactiles) choisit la consigne, le mode et le Boost ;
- la **carte de puissance** chauffe le fluide et fait tourner la soufflerie, d'après sa propre sonde de température. Elle mène le dialogue : elle **appelle** le boîtier de commande plusieurs fois par seconde, et il répond par son état.

La carte coupe le fil de données et se place entre les deux, comme un intermédiaire (*man in the middle*). Elle a **son propre état** (arrêt, marche à une consigne, Boost), qu'elle donne à la carte de puissance en répondant à ses appels. Face au boîtier de commande, elle joue la carte de puissance : elle l'appelle et lit ses réponses. Les touches et Home Assistant changent cet état ; le dernier ordre gagne.

```mermaid
flowchart LR
  HA[Home Assistant] <-- Wi-Fi --> D1[Wemos D1 mini<br>ESPHome]
  CMD[Boîtier de commande<br>écran, touches] <-- données --> D1
  D1 <-- données --> PWR[Carte de puissance<br>fluide, soufflerie]
  PWR -. 5 V et masse, en direct .- CMD
  PWR -. 5 V .-> D1
```

**Limite** : le boîtier de commande n'accepte aucun ordre par le bus. Son écran affiche toujours ce qui a été réglé sur ses touches : un Boost lancé par Home Assistant ne s'y voit pas.

## Le bus interne en bref

Le détail, trame par trame, est dans [`knowledge/linap.md`](knowledge/linap.md).

- Un seul fil de données, **9 600 bauds, 7 bits et parité impaire**. Les trames font **5 octets** : un en-tête, 3 octets de données, puis leur XOR.
- La carte de puissance envoie `75`, `C7` et `61` en boucle (cycle de 305 ms). Après `75` et `C7`, elle envoie l'octet `46` seul : c'est l'appel. Le boîtier de commande répond **dans le bit qui suit**, par 3 octets et la somme de contrôle :

| Réponse à `46` | Sens |
|---|---|
| `B0 cc 80` | en marche, consigne = `cc` sans son bit 7, divisé par 2 |
| `38 80 80` | Boost |
| `80 80 80` | arrêt |

- La carte de puissance donne son état dans `C7` : `80 E0 38` au repos, `01 E0 38` en chauffe, `83 E0 98` en Boost, `80 E0 40` à l'arrêt.
- **La carte de puissance obéit à quiconque répond à temps** à ses appels. **Le boîtier de commande ne se laisse pas piloter** : il ne répond qu'aux appels `46` et `70`, et aucune des quelque 280 trames essayées n'a changé sa consigne ni son écran.

## La découverte du protocole

Aucune documentation n'existe sur ce bus. Il a été décodé avec un **Wemos D1 mini** branché sur la nappe, en trois étapes. Les firmwares, le schéma du banc et les outils sont dans [`discovery/`](discovery/README.md).

| Étape | L'ESP… | Résultat |
|---|---|---|
| 1. Écoute | écoute la nappe entre les deux cartes | format des trames, consigne, Boost, arrêt |
| 2. Fausse carte de puissance | appelle le boîtier de commande seul, sur table, sans secteur | il répond et donne tout ce qu'on fait sur ses touches, mais n'accepte aucun ordre |
| 3. Fausse carte de commande | répond à la carte de puissance, sur le radiateur | elle suit la consigne, le Boost et l'arrêt |

> [!WARNING]
> **Le bus interne est au potentiel du secteur.** Pendant les essais sur le radiateur, l'ESP flotte à ce potentiel : alimentation de laboratoire à sortie flottante, pilotage en Wi-Fi uniquement, jamais d'USB ni d'appareil relié à la terre. Couper le disjoncteur avant de toucher au montage. En cas de doute, faites appel à un électricien.

## Le matériel

> [!NOTE]
> En cours : la plaque d'essai ([`hardware/`](hardware/README.md)) marche sur le radiateur ; reste le circuit imprimé ([backlog](knowledge/backlog.md)).

Ce qui ne changera pas :

- **Un Wemos D1 mini** (ESP8266), firmware ESPHome avec un composant en C++ : la réponse aux appels part de nos propres interruptions. Son antenne imprimée tient la liaison sur le radiateur, là où un ESP32-C3 super mini la perdait.
- **Alimentée par la nappe**, en principe : le 5 V de la carte de puissance tient ≈ 160 mA en tout, et un D1 mini avec le boîtier de commande en prend ≈ 120 (VCC à 5,11 V, démarrages du radiateur et Boost validés pendant la découverte). Reste à le confirmer avec la carte MITM, essayée jusqu'ici sur une alimentation de labo. Un condensateur de 470 µF près de l'ESP absorbe les pointes du Wi-Fi.
- **Sur chaque voie**, le montage du banc : un pont diviseur pour lire DATA, un transistor NPN pour la tirer à la masse.
- **Toute la carte est au potentiel du secteur**, comme le boîtier de commande : elle doit être entièrement enfermée, sans rien de touchable, et se mettre à jour par Wi-Fi seulement.

## Firmware

Le firmware de la carte est une configuration **ESPHome** pour le D1 mini, avec un composant en C++, dans [`firmware/`](firmware/README.md) : logique des trames testée sur ordinateur, réponse aux appels sur interruption ; Wi-Fi, API Home Assistant et OTA par ESPHome.

```sh
uv tool install esphome
cp discovery/esphome/secrets.example.yaml discovery/esphome/secrets.yaml   # Wi-Fi, clé API, mot de passe OTA
firmware/tests/run.sh
esphome run firmware/sauter-marapi.yaml        # premier flash en USB, avant tout câblage ; ensuite OTA
```

Les firmwares du banc, eux, sont des configurations ESPHome pour D1 mini, dans `discovery/` :

```sh
esphome run discovery/1-sniffer/linap-sniffer.yaml       # premier flash en USB, avant tout câblage
esphome logs discovery/1-sniffer/linap-sniffer.yaml | discovery/tools/decode.py
```

## Dans Home Assistant

Par l'intégration ESPHome (détail : [`firmware/README.md`](firmware/README.md)) :

| Entité | Usage |
|---|---|
| Mode | `off`, `on`, `boost` : l'état de la carte, changé par les touches ou par Home Assistant |
| Setpoint | la consigne |
| Power board, Heating | ce que fait la carte de puissance : attente, chauffe, Boost, arrêt |
| Boost duration | durée d'un Boost lancé par Home Assistant (30 min par défaut) |

Le dernier ordre gagne : un ordre de Home Assistant remplace celui des touches, et le prochain changement fait aux touches le remplace à son tour. Au démarrage, la carte reprend l'état enregistré.

Le boîtier de commande n'envoie que sa consigne effective : en Éco, Home Assistant voit la consigne Éco, sans savoir que c'est l'Éco.

## Limites connues

- **Écran non synchronisé** : ce que Home Assistant impose n'apparaît pas à l'écran.
- **Pas de température** pour l'instant : la carte de puissance a sa sonde, et une capture longue cherche dans quelle trame passe la mesure.
- **Garantie** : ouvrir le boîtier de commande fait perdre la garantie (2 ans).

## Pour aller plus loin

- [`knowledge/linap.md`](knowledge/linap.md) : le bus interne, trame par trame.
- [`knowledge/architecture.md`](knowledge/architecture.md) : rôle de la carte, solution retenue, pistes écartées, sécurité.
- [`knowledge/backlog.md`](knowledge/backlog.md) : ce qui reste à faire.
- `knowledge/manuals/` : notice Sauter.

---

**Mots-clés** : Sauter Marapi tactile, sèche-serviettes soufflant, radiateur électrique, Groupe Atlantic, Thermor, Atlantic, bus interne, liaison Linap, Err 10, man in the middle, rétro-ingénierie, ESPHome, composant externe C++, Home Assistant, domotique, ESP8266, Wemos D1 mini, boîtier de commande `HM011 TD1N01 RAD`, `74 60 5004`, STM32F091CCT6, circuit imprimé KiCad.
