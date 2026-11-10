# Bus Linap

Le **bus Linap** est la liaison interne du sèche-serviettes, une nappe de 3 fils entre le **boîtier de commande** (écran et touches, appelé ici *carte de commande*) et la **carte de puissance** (résistance, soufflerie, sonde). Quand elle est rompue, l'écran affiche « Err 10 ». Aucune documentation publique n'existe : tout ce qui suit a été mesuré sur le radiateur (méthode et outils : [`discovery/`](../discovery/README.md)).

## En bref

- Un fil de données, **9 600 bauds, 7 bits + parité impaire**, trames de **5 octets** avec un XOR en somme de contrôle.
- La **carte de puissance mène** : elle envoie ses trames en boucle et **appelle** la carte de commande par un octet seul, `46`. La carte de commande complète la trame 1 bit plus tard avec son état : **consigne, Boost ou arrêt**.
- **La carte de puissance obéit à n'importe qui** qui répond correctement à ses appels : consigne, Boost, arrêt, vérifié avec un ESP à la place de la carte de commande. C'est elle qui décide de chauffer, d'après sa propre sonde.
- **La carte de commande ne se laisse pas piloter** : elle répond à un ESP qui joue la carte de puissance (on lit donc tout ce que fait l'utilisateur), mais rien de ce qu'on lui envoie ne change sa consigne, son mode ni son écran.
- Conséquence : un **ESP au milieu (MITM)** peut lire les touches et piloter le radiateur, mais **l'écran ne montrera pas ce que décide Home Assistant**. C'est la solution retenue (`architecture.md`).

## Couche physique

| Fil | Rôle | Mesure |
|---|---|---|
| noir | GND | — |
| rouge | VCC, fourni par la carte de puissance | 5,22 V |
| blanc | DATA, au repos à l'état haut (≈ VCC − 0,3 V) | — |

- **Le 5 V de la nappe** (essai de charge du 10/11/2026, résistances de 220 Ω ajoutées au boîtier de commande, qui prend 35 à 40 mA) : il tient **≈ 160 mA en tout**, démarrage, touches et Boost compris (5,10 V). Vers 180 mA il tient au repos (5,08 V) mais lâche dès qu'on touche l'écran ou au démarrage ; vers 205 mA il lâche en quelques secondes. La tension ne baisse presque pas avant de s'effondrer : c'est une limite de courant, pas une chute progressive. Quand elle lâche, le boîtier de commande redémarre en boucle et le bus se tait.
- Chaque carte tire DATA à la masse pour émettre (collecteur ouvert). Le rappel à l'état haut vient de la carte de commande : sans elle, il faut une résistance (5 kΩ vers VCC sur le banc).
- **Le bus est au potentiel du secteur** : entre GND et le neutre, 850 Ω / 1117 Ω selon le sens (0,32 V / 0,72 V en test de diode), OL vers la phase. Rien de relié à la terre ou à un PC ne doit s'y brancher.

## Octets et trames

- **9 600 bauds, 7 bits de données + parité impaire + 1 bit de stop** : une UART 8N1 lit chaque octet avec son bit 7 qui complète à un nombre impair de 1. Vérifié sur les 7 000 trames de la capture guidée, sans exception.
- **Trame de 5 octets** : en-tête, 3 octets de données, puis le XOR des 3 octets de données (qui a lui aussi la bonne parité).
- Dans les trames de la carte de puissance, ≈ 7 bits de repos après l'en-tête puis 2 bits entre les octets.

## Le dialogue

Cycle de ≈ 305 ms, en 5 créneaux de ≈ 61 ms, toujours mené par la carte de puissance :

```
75 ··· [46 + réponse] ······ C7 ··· [46 + réponse] ······ 61 ······ (75 …)
```

- ≈ 13 ms après `75` et après `C7`, la carte de puissance envoie **l'octet d'appel `46` seul** (toutes les 30 s environ, `70` à la place après un `75`).
- La carte de commande répond **1 bit après** l'appel, avec les 3 octets de données et la somme de contrôle, enchaînés et sans en-tête. Sur le fil, l'appel et la réponse forment donc une trame `46` complète.
- De temps en temps, et après chaque changement de la réponse `46`, la carte de puissance glisse la rafale `73`, `67`, `76` dans le cycle ; `76` est suivi d'un appel.
- **Tolérances mesurées** :
  - la carte de commande répond à un appel envoyé de 2 à 45 ms après la trame précédente, et ne répond à aucune autre adresse (`00` à `7F` essayés) ;
  - la carte de puissance ignore une réponse qui arrive 2 ms après son appel : il faut répondre dans le bit qui suit ;
  - la carte de puissance tolère des appels sans réponse (un sur deux servi, sans effet) ; 30 s sans aucune réponse ne changent rien, au-delà ce n'est pas mesuré ;
  - seule, sans appel, la carte de commande se tait, et affiche « Err » au bout de quelques minutes.

## Trames connues

Octets donnés avec leur bit de parité, tels qu'on les lit sur le fil.

| En-tête | Émetteur | Données | Sens |
|---|---|---|---|
| `46` | appel : carte de puissance ; données : carte de commande | `B0 cc 80` | marche, consigne = `cc` & 0x7F ÷ 2 (14,5 → `9D`, 17 → `A2`, 20,5 → `29`) |
| | | `38 80 80` | Boost demandé |
| | | `80 80 80` | arrêt (aussi au démarrage, avant la consigne) |
| `70` | idem, toutes les 30 s | `83 80 89` | toujours la même, sens inconnu |
| `C7` | carte de puissance | `80 E0 38` | en marche, ne chauffe pas |
| | | `01 E0 38` | chauffe du fluide |
| | | `83 E0 98` | Boost (soufflerie) |
| | | `83 E0 38` | brièvement, à la fin d'un Boost |
| | | `80 E0 40` | arrêt |
| `67` | carte de puissance, après un changement | `80 80 80` / `1F 40 80` | sans chauffe / chauffe ou Boost |
| `75` | carte de puissance | `83 xx 02` | `xx` varie en continu (de 0 à 127 pendant le Boost) : mesure ou compteur, inconnu |
| `61` | carte de puissance | `1C xx 80` … | varie, inconnu |
| `73` | carte de puissance, après un changement | `37 45 02` … | octet 1 entre 55 et 59 : candidate pour une température, inconnu |
| `76` | carte de puissance, ferme la rafale | `02 80 89` | inconnu |

- La consigne est la **consigne effective** : en Éco, la carte de commande envoie la consigne Éco, sans dire qu'elle est en Éco. Elle ne l'envoie qu'une fois la saisie finie (≈ 5 s après la dernière touche).
- La durée du Boost (menu expert) et le verrouillage des touches ne passent pas sur le bus : la carte de commande gère son minuteur et n'envoie que « Boost » ou « pas Boost ».

## Qui décide quoi

| | Carte de commande | Carte de puissance |
|---|---|---|
| Consigne, Boost, arrêt, programme | **décide** (touches), les envoie dans `46` | exécute ce qu'elle reçoit dans `46`, sans vérifier l'émetteur |
| Chauffer ou non | — | **décide**, en comparant la consigne à sa propre sonde (`C7` passe à `01`) |
| Écran | n'affiche que son propre état (consigne, Prog, Éco / Confort / Hors-gel, Boost) | aucune influence : `C7` en Boost, chauffe ou arrêt ne change rien à l'écran |

Essais d'écriture vers la carte de commande (au banc, sans secteur) : chacun des 3 octets de données des trames `C7`, `75`, `61`, `67`, `73`, `76` mis à `00`, à chaque bit seul puis à `7F`, et 120 en-têtes inconnus. **Une seule réaction** : `C7 80 40 38` (octet 2 à `40` au lieu de `E0`) lui fait répondre une fois `46 80 80 80 80`, comme à un redémarrage de la carte de puissance, puis elle revient à son état. Rien d'autre, ni sur le bus, ni à l'écran.

## Ce que ça implique pour le MITM

- **DATA coupé en deux**, GND et VCC traversent. L'ESP joue la carte de puissance côté commande (il appelle, elle répond) et la carte de commande côté puissance (il répond aux appels).
- **Lecture** : chaque réponse de la carte de commande donne la consigne, le Boost ou l'arrêt choisis aux touches. `C7` dit si le radiateur chauffe.
- **Écriture** : l'ESP répond ce qu'il veut à la carte de puissance. Par défaut il relaie la carte de commande ; quand Home Assistant impose un état, il l'envoie à la place, jusqu'au prochain changement fait aux touches. L'écran, lui, garde l'état choisi aux touches.
- **Temps réel** : la réponse doit partir dans le bit qui suit l'appel (≈ 100 µs). Il faut une réception sur interruption ou par UART matérielle, pas une lecture dans une boucle principale.
- **Panne** : si l'ESP tombe, les deux moitiés de DATA ne se parlent plus et le radiateur s'arrête en « Err 10 », sans chauffer. Un redémarrage de l'ESP (une dizaine de secondes) passe sans effet.

## Pas encore connu

- Le sens de `75`, `61`, `73`, `76` et de la réponse `70`, et où passe la température mesurée par la carte de puissance : une capture longue avec une consigne proche de la température de la pièce devrait le montrer.
- Ce que fait la carte de puissance après un long silence de la carte de commande.

## Sécurité

- **Le bus est au potentiel du secteur** : couper au tableau avant de toucher au boîtier de commande ou à la nappe.
- Tout ce qui s'y branche flotte à ce potentiel : jamais d'USB, d'appareil relié à la terre ni de PC dessus. Les ESP s'y flashent en USB avant d'être câblés, puis se mettent à jour en Wi-Fi.
- La garantie (2 ans) est perdue dès qu'on ouvre le boîtier de commande.
