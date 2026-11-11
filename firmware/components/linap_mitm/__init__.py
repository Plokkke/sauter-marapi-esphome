"""The Linap MITM of the Sauter Marapi radiator, on a Wemos D1 mini (ESP8266): see linap_mitm.h."""

import esphome.codegen as cg
import esphome.config_validation as cv
from esphome.const import CONF_ID

AUTO_LOAD = ["remote_base"]

linap_mitm_ns = cg.esphome_ns.namespace("linap_mitm")
LinapMitm = linap_mitm_ns.class_("LinapMitm", cg.Component)

CONFIG_SCHEMA = cv.All(
    cv.Schema({cv.GenerateID(): cv.declare_id(LinapMitm)}).extend(cv.COMPONENT_SCHEMA),
    cv.only_on_esp8266,
)


async def to_code(config):
    var = cg.new_Pvariable(config[CONF_ID])
    await cg.register_component(var, config)
