# Firmware du MITM

Firmware Rust de la carte MITM : un ESP32-C3 sur la nappe coupée, entre la carte de commande et la carte de puissance (`knowledge/architecture.md`). Il relaie le boîtier de commande ou impose ce que décide Home Assistant, par MQTT.

| Dossier | Contenu |
|---|---|
| [`linap/`](linap/) | la logique, en Rust pur (`no_std`, sans E/S, le temps vient de l'appelant), testée sur le Mac : trames, parité, consigne, états des deux cartes, découpage des trames sur le fil, réponse aux appels, état imposé |
| [`mitm/`](mitm/) | le firmware de l'ESP32-C3 (`esp-hal`, `embassy`) : les deux UART servies sur interruption, le Wi-Fi, MQTT et la découverte Home Assistant |

## Ce que fait la carte

- **Côté carte de puissance**, elle joue la carte de commande : à chaque octet d'appel (`46`, ou `70`), elle répond depuis l'interruption de réception, 1 bit après, avec les 4 octets attendus.
- **Côté carte de commande**, elle joue la carte de puissance : elle répète octet par octet tout ce que dit la carte de puissance, appels compris, et lit les réponses de la carte de commande.
- **Qui a la main** : le dernier ordre gagne.
  - Par défaut, la réponse envoyée à la carte de puissance est la dernière réponse de la carte de commande (mode `relay`).
  - Un ordre de Home Assistant (consigne, Boost, arrêt) l'impose à la place.
  - Le prochain **changement** de la réponse de la carte de commande rend la main aux touches. Home Assistant en est informé tout de suite : le sélecteur repasse sur `relay`, les capteurs des touches changent.
  - Sans broker MQTT pendant 30 min, la carte repasse aussi en `relay`.
- La carte de commande répète son état à chaque appel, environ 6 fois par seconde. Seul un changement compte, la répétition ne reprend donc pas la main. Mais tout changement compte, qu'il vienne ou non d'un doigt :
  - le programme (Prog) qui passe de Confort à Éco change la consigne effective ;
  - la fin d'un Boost lancé aux touches (minuteur local) ;
  - un redémarrage de la carte de commande (`80 80 80`, puis la consigne).
- À l'inverse, une touche qui ne change rien à l'état (remettre la même consigne) ne se voit pas.
- **Boost imposé par HA** : la carte de commande ne le voit pas, et son minuteur ne l'arrête donc pas. La carte MITM l'arrête elle-même au bout de « Boost duration » (30 min par défaut) et repasse en `relay`, même si HA ou le Wi-Fi sont tombés entre-temps. Pour prolonger, HA impose `boost` de nouveau (`select.select_option`), ce qui relance la durée entière.

## Home Assistant

Découverte MQTT en un seul message (`homeassistant/device/sauter_marapi/config`), état en JSON sur `sauter-marapi/state` à chaque changement et toutes les 30 s, disponibilité sur `sauter-marapi/availability`.

| Entité | Type | Rôle |
|---|---|---|
| Keys mode | capteur | `off`, `boost`, `on` : l'état réglé aux touches |
| Keys setpoint | capteur, °C | la consigne réglée aux touches (effective : celle de l'Éco en Éco) |
| Power board | capteur | `C7` : `idle`, `heating`, `boost`, `boost_ending`, `off`, `unknown` |
| Heating | capteur binaire | la résistance ou la soufflerie tourne |
| Control | `select` | `relay` (les touches), `setpoint`, `boost`, `off` (imposés) |
| Setpoint | `number`, 5 à 30 °C | la consigne en vigueur ; la régler l'impose |
| Boost duration | `number`, 1 à 120 min (configuration) | durée d'un Boost imposé, 30 min tant que HA n'a rien réglé |
| Reply gap | `number`, bits (configuration) | silence entre l'appel et la réponse, 1 bit par défaut : sert à mesurer la marge de la carte de puissance |
| Polls answered | capteur, % (diagnostic) | part des appels auxquels la carte a eu de quoi répondre |
| Frame log | interrupteur (configuration) | publie les trames du bus sur `sauter-marapi/frames` |
| Note | texte (diagnostic) | écrit un repère dans le journal des trames |

Les réglages (Boost duration, Reply gap, Frame log) sont publiés en *retain* : Mosquitto garde la dernière valeur choisie dans HA et la rend à la carte dès qu'elle se reconnecte, après un redémarrage ou une mise à jour. La carte ne stocke rien en flash. Les ordres (Control, Setpoint) ne sont pas gardés : un redémarrage rend la main aux touches.

## Journal des trames

La carte remplace le sniffer de `discovery/` : elle entend tout le bus, les trames de la carte de puissance sur une voie, les réponses de la carte de commande sur l'autre. Avec « Frame log » allumé, elle publie sur `sauter-marapi/frames` chaque trame qui diffère de la précédente de même en-tête, sur sa voie :

```
812.304 P C7 01 E0 38 D9     secondes depuis le démarrage, P = voie de la carte de puissance
812.418 C 46 B0 A2 80 92     C = réponse de la carte de commande
815.002 note consigne 21     repère écrit dans le champ « Note » de HA
```

Sur la voie P, les trames `46` et `70` sont nos propres réponses. Pour capturer sur le Mac (`brew install mosquitto`) :

```sh
set -a; . firmware/secrets.env; set +a
mosquitto_sub -h "$MQTT_HOST" -u "$MQTT_USER" -P "$MQTT_PASSWORD" -t sauter-marapi/frames | tee capture.log | grep -v " 75 "
```

Dans HA, *Paramètres > Appareils et services > MQTT > Configurer > Écouter un sujet* affiche aussi le flux. Les journaux du firmware lui-même (`log`) ne sortent que par l'USB-JTAG, donc au bureau seulement.

## Brochage

| Voie | UART | RX (pont diviseur) | TX (NPN, sortie inversée) |
|---|---|---|---|
| carte de puissance | UART0 | GPIO4 | GPIO5 |
| carte de commande | UART1 | GPIO6 | GPIO7 |

Sur un XIAO ESP32-C3 : D2, D3, D4, D5. Les journaux passent par l'USB-JTAG, jamais par une UART du bus. La base des NPN doit avoir une résistance de rappel à la masse, sans quoi DATA risque d'être tiré à l'état bas au démarrage.

## Construire et flasher

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh   # Rust stable, la cible RISC-V vient de rust-toolchain.toml
cargo install espflash --locked
cp firmware/secrets.example.env firmware/secrets.env              # Wi-Fi et compte MQTT, lus à la compilation
(cd firmware/linap && cargo test)                                 # la logique, sur le Mac
(cd firmware/mitm && cargo run --release)                         # compile, flashe en USB et affiche les journaux
```

**Flasher en USB avant tout câblage**, jamais une fois la carte sur la nappe : elle est au potentiel du secteur (`discovery/README.md`). Pas encore d'OTA : pour le moment, toute mise à jour repasse par le bureau.
