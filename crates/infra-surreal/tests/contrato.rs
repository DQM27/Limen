//! La batería de contrato de la persistencia, contra `SurrealDB` embebido
//! en memoria (`kv-mem`): una base nueva por prueba.

#[cfg(test)]
mod tests {
    use limen_infra_surreal::AlmacenSurreal;

    limen_pruebas_contrato::bateria_de_contrato!(AlmacenSurreal::en_memoria().await.unwrap());
}
