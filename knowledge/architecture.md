# Architecture

## Rôles

| Niveau | Projet | Rôle |
|---|---|---|
| Sèche-serviettes | Sauter Marapi tactile 1750 W (750 W de fluide + 1000 W de soufflerie) | sa carte de commande (écran, touches) choisit consigne, mode et Boost ; sa carte de puissance chauffe d'après sa propre sonde |
| Pont | **ce projet** | un Wemos D1 mini, firmware ESPHome, sur le bus Linap interne : tient l'état du radiateur, que changent les touches et Home Assistant |
| Maison | automatisation HA (autre projet) | plannings, présence, délestage |

Ce projet ne contient aucune automatisation HA.

## Solution retenue : un ESP au milieu du bus Linap

**Retenu** (11/2026) : un **MITM** sur la nappe entre la carte de commande et la carte de puissance (protocole : `linap.md`). **Validé sur le radiateur le 12/11/2026** avec la plaque d'essai (`hardware/`) : les touches et Home Assistant pilotent la carte de puissance.

- DATA est coupé, GND et VCC traversent. L'ESP joue la carte de puissance face à la carte de commande, et la carte de commande face à la carte de puissance.
- **L'écran et les touches d'origine restent** : pas d'interface à refaire.
- **Deux émulateurs autonomes** (décidé le 11/11/2026) : chacun tourne sur sa propre horloge et marche même sans l'autre carte. Rien n'est relayé octet par octet, il n'y a ni mode relais ni écho.
- **Le MITM a son propre état** (décidé le 11/11/2026) : arrêt, marche à une consigne, ou Boost. Il l'envoie à la carte de puissance en réponse à chaque appel `46`. Les touches (lues dans les réponses du boîtier de commande) et Home Assistant le changent ; **le dernier ordre gagne**.
- **Au démarrage** : l'état enregistré en flash ; arrêt à 15 °C la toute première fois. La première réponse du boîtier de commande n'est qu'une référence quand un état a été restauré : seul un changement ultérieur aux touches compte comme un ordre.
- **Limite acceptée : l'écran n'est pas synchronisé.** La carte de commande n'affiche que ce qu'on règle sur ses touches ; un Boost lancé par HA ne s'y voit pas. HA, lui, sait tout : réglages faits aux touches, chauffe en cours.
- **Contrepartie** des émulateurs autonomes : une carte de puissance en panne ne fait plus afficher « Err 10 » au boîtier de commande.

Écartés :

- **Fil pilote + touche Boost simulée** : une carte qui pilotait le radiateur par son fil pilote 6 ordres et déclenchait le Boost par un faux doigt (photoMOS et condensateur sur l'électrode de la touche). Elle fonctionnait sans rien connaître du radiateur, mais sans retour d'état, et le Boost passait par un artifice. Jamais fabriquée ; schéma, circuit imprimé, firmware et notes sont dans l'historique git (supprimés après le commit `c7eed15`).
- **Contrôleur maison à la place de la carte de commande** : écran toujours juste, mais toute l'interface (touches, affichage, programme) à refaire. Reste le repli si le MITM échoue.
- **Relais par défaut, état imposé par HA jusqu'au prochain appui** (première conception, 11/11/2026) : remplacé par l'état propre du MITM, plus simple et testable avec un seul côté branché.

## Liaison avec Home Assistant

**Décidé** (11/11/2026) : **l'API native d'ESPHome**, chiffrée, à la place de MQTT. Entités : [`firmware/README.md`](../firmware/README.md).

- L'état : mode (`off`, `on`, `boost`) et consigne, que HA lit et règle.
- La carte de puissance : état `C7` et chauffe en cours.
- Les touches ne sont pas exposées à part : un changement aux touches change l'état, que HA voit aussitôt. La carte de commande n'envoie que sa consigne effective : en Éco, HA voit la consigne Éco, sans le mot « Éco ».
- **Ce qui compte comme un ordre des touches** : tout changement de la réponse `46`. Le programme (passage Confort ↔ Éco) et la fin d'un Boost lancé aux touches changent donc l'état aussi.
- **Un Boost lancé par HA s'arrête sur la carte MITM** (décidé le 11/11/2026). Le minuteur du Boost est dans la carte de commande, qui ne voit pas ce Boost : le MITM revient à l'état d'avant au bout d'une durée réglable dans HA (30 min par défaut), même sans HA. HA prolonge en relançant le Boost.

## Sécurité

- **Sans Home Assistant** : le MITM garde son état et les touches continuent de le changer ; un Boost lancé par HA s'arrête de lui-même. Le radiateur se comporte comme un radiateur réglé aux touches.
- **Pas de relais de repli** (décidé le 10/11/2026) : si la carte elle-même tombe en panne, le radiateur s'arrête en « Err 10 », sans chauffer, donc sans danger. La carte se branche par des connecteurs : la débrancher et reconnecter la nappe d'origine remet le radiateur dans son état d'usine.
- **Démarrage et mise à jour OTA** : une dizaine de secondes sans dialogue, tolérées (30 s mesurées sans effet côté carte de puissance, plusieurs minutes avant « Err » côté carte de commande).
- **Potentiel du secteur** : toute la carte est au potentiel du bus, donc du secteur, comme le boîtier de commande. Elle est entièrement enfermée, sans partie touchable ni USB accessible, et se met à jour en OTA uniquement.

## Firmware

**Décidé** (11/11/2026) : **ESPHome sur Wemos D1 mini (ESP8266), avec un composant externe en C++** (`firmware/components/linap_mitm/`). Remplace la décision du 10/11/2026 (Rust sur ESP32-C3).

- **Pourquoi plus le C3** : sur le radiateur, le Wi-Fi du C3 super mini décrochait (broker MQTT perdu au bout de 90 s), alors que le D1 mini tient la liaison au même endroit.
- **Pourquoi plus Rust** : il ne prend pas en charge l'ESP8266 (`esp8266-hal` archivé, pas de cible Xtensa LX106), et le mélanger au C passerait par le LLVM d'Espressif, une cible maison et `build-std`.
- **Pourquoi ESPHome malgré le temps réel** : il apporte Wi-Fi, API, OTA et journaux sans rien écrire. Tout ce qui est critique tourne dans nos propres interruptions, donc la boucle d'ESPHome ne compte pas :
  - voie carte de puissance : l'UART0, déplacée sur D7/D8, avec notre interruption de réception, qui envoie la réponse à l'appel dans le bit qui suit ;
  - voie boîtier de commande : l'émission bit à bit depuis l'interruption NMI du timer1 sur D6 (l'UART1 n'a pas de réception, et D4, sa seule sortie, doit rester haut au démarrage) ; la réception par `remote_receiver` sur D5.
- La logique (trames, parité, état, émulateur de carte de puissance) est du C++ pur, testé sur le Mac (`firmware/tests/`) ; le temps vient de l'appelant.
- Les journaux passent par l'API seulement (`logger: baud_rate: 0`), l'UART0 servant la voie carte de puissance.

## Alimentation

**Décidé** (10/11/2026) : la carte s'alimente sur le **5 V de la nappe**, comme le boîtier de commande. Plus de raccordement au 230 V, ni HLK-PM01, ni fusible, ni varistance. **À confirmer avec la carte MITM** : pendant ses essais (11-12/11/2026), le D1 et le boîtier de commande étaient sur une alimentation de laboratoire flottante.

- Le 5 V de la carte de puissance tient ≈ 160 mA en tout et lâche vers 180 mA (`linap.md`). Le boîtier de commande en prend 40, un D1 mini en Wi-Fi 80 en moyenne : ≈ 120 mA, ≈ 40 mA de marge.
- Validé pendant la découverte avec un D1 mini sur la nappe et le boîtier de commande branché : VCC à 5,11 V, plusieurs démarrages du radiateur, navigation et Boost.
- Le reste de la carte MITM (ponts, transistors, rappels) prend 1 à 2 mA en moyenne. C1 (470 µF) ne coûte que sa charge à la mise sous tension ; il absorbe les pointes d'émission Wi-Fi.
- Pas de marge pour une bobine de relais : la carte reste sobre.
- Sur chaque voie, pas besoin d'optocoupleur : l'ESP est au potentiel du bus, et le montage du banc suffit (pont diviseur en lecture, NPN en émission).
