# Firmware de la carte MITM

Une configuration ESPHome pour le Wemos D1 mini ([`sauter-marapi.yaml`](sauter-marapi.yaml)) et un composant externe en C++ ([`components/linap_mitm/`](components/linap_mitm/)). Choix et raisons : [`knowledge/architecture.md`](../knowledge/architecture.md). Carte : [`hardware/`](../hardware/README.md). Protocole : [`knowledge/linap.md`](../knowledge/linap.md).

## Ce qu'il fait

- **Il tient l'état du radiateur** : `off`, `on` à une consigne, ou `boost`. Home Assistant et les touches le changent, le dernier ordre gagne. L'état est enregistré en flash ; la toute première fois, arrêt à 15 °C.
- **Il joue le boîtier de commande face à la carte de puissance** : il répond à chaque appel `46` par l'état, et à `70` par sa réponse constante.
- **Il joue la carte de puissance face au boîtier de commande** : il envoie le cycle `75`, `C7`, `61` toutes les 61 ms et ses appels, puis lit les réponses. Un changement de la réponse `46` est un ordre des touches.
- Chaque côté marche seul, même sans l'autre carte.

## Broches

| D1 | GPIO | Voie | Rôle |
|---|---|---|---|
| D7 | 13 | carte de puissance | réception, UART0 déplacée, par le pont R1/R2 |
| D8 | 15 | carte de puissance | émission, UART0 déplacée et inversée, vers Q1 |
| D5 | 14 | boîtier de commande | réception, `remote_receiver`, par le pont R6/R7 |
| D6 | 12 | boîtier de commande | émission bit à bit depuis le NMI du timer1, vers Q2 |

Le firmware coupe le rappel interne de D7, que le core Arduino active pour l'UART0 déplacée : avec lui, le pont ne descend plus sous ≈ 1,8 V et aucun octet n'est reçu.

## Les fichiers

| Fichier | Rôle |
|---|---|
| `linap.h` | trames, parité, consigne, état du boîtier et de la carte de puissance, découpage des octets en trames (pur) |
| `mitm.h` | l'état et ses règles : ordres, restauration, Boost borné (pur) |
| `power_board.h` | le cycle de l'émulateur de carte de puissance (pur) |
| `power_side.*` | voie carte de puissance : UART0 et son interruption, qui répond dans le bit qui suit l'appel |
| `commander_tx.*` | voie boîtier de commande : émission par le NMI du timer1 |
| `linap_mitm.*` | le composant ESPHome : relie le tout, enregistre l'état, journalise |

Le code pur se teste sur l'ordinateur : `tests/run.sh` (27 tests).

## Dans Home Assistant

Intégration ESPHome, avec la clé `api_encryption_key` de `discovery/esphome/secrets.yaml`.

| Entité | Usage |
|---|---|
| Mode | `off`, `on`, `boost` : l'état, qu'on lit et qu'on règle |
| Setpoint | la consigne, de 5 à 30 °C par demi-degré |
| Power board, Heating | ce que fait la carte de puissance (`C7`) : attente, chauffe, Boost, arrêt |
| Boost duration | durée d'un Boost lancé par HA, 30 min par défaut ; relancer le Boost le prolonge |
| Power board polls answered, Commander polls answered | santé des deux voies, en % sur 10 s |
| Reply gap | délai entre l'appel et la réponse, en bits (1 comme le vrai boîtier) |
| Frame log | journalise chaque trame qui change, des deux côtés |

Toutes les 10 s, le journal donne pour chaque voie les octets entendus, les appels et les réponses complètes.

## Construire et flasher

```sh
uv tool install esphome
cp discovery/esphome/secrets.example.yaml discovery/esphome/secrets.yaml   # Wi-Fi, clé API, mot de passe OTA
firmware/tests/run.sh
esphome run firmware/sauter-marapi.yaml        # premier flash en USB, AVANT tout câblage ; ensuite OTA
esphome logs firmware/sauter-marapi.yaml       # journaux par l'API
```

Sur le radiateur, toute la carte est au potentiel du secteur : jamais d'USB, mises à jour en OTA seulement.

Pour tester sans radiateur ni secteur : boîtier de commande débranché, relier la broche 1 de J1 à celle de J2. Le D1 se répond à lui-même : 65 appels sur 65 toutes les 10 s des deux côtés.
