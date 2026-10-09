//! La batería de contrato de la persistencia, contra el doble en memoria.

#[cfg(test)]
mod tests {
    use limen_infra_memoria::AlmacenMemoria;

    limen_pruebas_contrato::bateria_de_contrato!(AlmacenMemoria::new());
}
