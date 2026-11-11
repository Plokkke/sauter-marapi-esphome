# Architecture

## Rôles

| Niveau | Projet | Rôle |
|---|---|---|
| Sèche-serviettes | Sauter Marapi tactile 1750 W (750 W de fluide + 1000 W de soufflerie) | sa carte de commande (écran, touches) choisit consigne, mode et Boost ; sa carte de puissance chauffe d'après sa propre sonde |
| Pont | **ce projet** | un ESP32-C3, firmware en Rust, sur le bus Linap interne : lit ce que fait l'utilisateur, impose ce que décide Home Assistant |
| Maison | automatisation HA (autre projet) | plannings, présence, délestage |

Ce projet ne contient aucune automatisation HA.

## Solution retenue : un ESP au milieu du bus Linap

**Retenu** (11/2026) : un **MITM** sur la nappe entre la carte de commande et la carte de puissance (protocole : `linap.md`).

- DATA est coupé, GND et VCC traversent. L'ESP joue la carte de puissance face à la carte de commande, et la carte de commande face à la carte de puissance : les deux moitiés ont été validées séparément (`discovery/`).
- **L'écran et les touches d'origine restent** : pas d'interface à refaire.
- **Par défaut, l'ESP relaie** : le radiateur se comporte exactement comme sans lui.
- **Deux émulateurs autonomes** (décidé le 11/11/2026) : l'ESP joue la carte de puissance face au boîtier sur sa propre horloge, et le boîtier face à la carte de puissance ; ils ne partagent que des états (réponse `46`, état `C7`), rien n'est relayé octet par octet. Plus simple, testable avec un seul des deux côtés ; en contrepartie, une carte de puissance en panne ne fait plus afficher « Err 10 ».
- **Home Assistant peut imposer** une consigne, le Boost ou l'arrêt : l'ESP l'envoie à la carte de puissance à la place de l'état de la carte de commande, jusqu'au prochain changement fait aux touches, qui reprend la main. **Le dernier ordre gagne**, et HA voit aussitôt chaque changement fait aux touches.
- **Limite acceptée : l'écran n'est pas synchronisé.** La carte de commande n'affiche que ce qu'on règle sur ses touches ; un Boost lancé par HA ne s'y voit pas. HA, lui, sait tout : réglages faits aux touches, chauffe en cours.

Écartés :

- **Fil pilote + touche Boost simulée** : une carte qui pilotait le radiateur par son fil pilote 6 ordres et déclenchait le Boost par un faux doigt (photoMOS et condensateur sur l'électrode de la touche). Elle fonctionnait sans rien connaître du radiateur, mais sans retour d'état, et le Boost passait par un artifice. Jamais fabriquée ; schéma, circuit imprimé, firmware et notes sont dans l'historique git (supprimés après le commit `c7eed15`).
- **Contrôleur maison à la place de la carte de commande** : écran toujours juste, mais toute l'interface (touches, affichage, programme) à refaire. Reste le repli si le MITM échoue.

## Liaison avec Home Assistant

**Décidé** (11/11/2026) : **MQTT avec la découverte automatique**, sur le Mosquitto de Home Assistant. Une seule annonce pour tout l'appareil, un état en JSON, des commandes par entité. Entités et sujets : [`firmware/README.md`](../firmware/README.md).

- Les touches : mode et consigne réglés, lus dans la réponse `46`. La carte de commande n'envoie que sa consigne effective : HA voit la consigne Éco quand elle est en Éco, sans le mot « Éco ».
- La carte de puissance : état `C7` et chauffe en cours.
- Le contrôle : `relay` (les touches) ou un état imposé (`setpoint`, `boost`, `off`), et la consigne en vigueur, qu'il suffit de régler pour l'imposer.
- **Ce qui reprend la main sans doigt** : tout changement de la réponse `46` compte comme un ordre local. Le programme (passage Confort ↔ Éco), la fin d'un Boost lancé aux touches et un redémarrage de la carte de commande rendent donc la main aux touches.
- **Un Boost imposé par HA s'arrête sur la carte MITM** (décidé le 11/11/2026). Le minuteur du Boost est dans la carte de commande, qui ne voit pas ce Boost : la carte MITM rend donc la main aux touches au bout d'une durée réglable dans HA (30 min par défaut), même sans HA. HA prolonge en imposant le Boost de nouveau.

## Sécurité

- **Sans Home Assistant** : l'ESP reste en mode transparent, et y revient s'il a perdu le broker MQTT depuis 30 min alors qu'un état était imposé (le broker tient lieu de HA : il tourne sur le même serveur). C'est le seul risque à couvrir.
- **Pas de relais de repli** (décidé le 10/11/2026) : si la carte elle-même tombe en panne, le radiateur s'arrête en « Err 10 », sans chauffer, donc sans danger. La carte se branche par des connecteurs : la débrancher et reconnecter la nappe d'origine remet le radiateur dans son état d'usine.
- **Démarrage et mise à jour OTA** : une dizaine de secondes sans dialogue, tolérées (30 s mesurées sans effet côté carte de puissance, plusieurs minutes avant « Err » côté carte de commande).
- **Potentiel du secteur** : alimentée par la nappe, toute la carte est au potentiel du bus, donc du secteur, comme le boîtier de commande. Elle est entièrement enfermée, sans partie touchable ni USB accessible, et se met à jour en OTA uniquement.

## Firmware

**Décidé** (10/11/2026) : **Rust sur ESP32-C3**, plutôt qu'ESPHome.

- Le C3 a deux UART complètes, une par voie, et Rust (`esp-hal`, `embassy`) permet de répondre à l'appel sur interruption, dans le bit qui suit, sans dépendre d'une boucle principale.
- La logique (trames, parité, relais, état imposé) est du code pur, testable sur le Mac ; le temps vient de l'appelant.
- ESPHome ne sert plus qu'au banc (`discovery/`). Ce qu'il donnait sans rien écrire est refait dans `firmware/mitm/` : Wi-Fi et liaison MQTT ; reste l'OTA.
- Écrit le 11/11/2026 (`firmware/`) : la crate `linap` (29 tests sur le Mac) et le firmware, qui compile (image de 594 ko). Les deux voies sont servies par l'interruption de réception de leur UART, à la priorité maximale.
- L'ESP8266 du D1 mini est écarté : Rust ne le prend plus en charge (ni HAL maintenue, ni pile Wi-Fi).
- Antenne : un C3 vendu avec une antenne externe (Seeed XIAO ESP32-C3 par exemple, à vérifier), le radiateur étant loin de la box.

## Alimentation

**Décidé** (10/11/2026) : la carte s'alimente sur le **5 V de la nappe**, comme le boîtier de commande. Plus de raccordement au 230 V, ni HLK-PM01, ni fusible, ni varistance. **À confirmer avec le C3** : la mesure a été faite avec un D1 mini.

- Le 5 V de la carte de puissance tient ≈ 160 mA en tout et lâche vers 180 mA (`linap.md`). Le boîtier de commande en prend 40, un D1 mini en Wi-Fi 80 en moyenne : ≈ 120 mA, ≈ 40 mA de marge. Un C3 en Wi-Fi est du même ordre, avec des pointes d'émission plus fortes : à mesurer sur la nappe comme le D1.
- Validé avec un D1 mini sur la nappe et le boîtier de commande branché : VCC à 5,11 V, plusieurs démarrages du radiateur, navigation et Boost.
- Un condensateur de 470 à 1000 µF près de l'ESP absorbe les pointes d'émission Wi-Fi.
- Pas de marge pour une bobine de relais : la carte reste sobre.
- Sur chaque voie, pas besoin d'optocoupleur : l'ESP est au potentiel du bus, et le montage du banc suffit (pont diviseur en lecture, NPN en émission).
