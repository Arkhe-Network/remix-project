#!/bin/bash
# Script para implantar com configuração PQC

ENV=${1:-staging}
case $ENV in
    staging)
        export CATHEDRAL_SIGNATURE_ALG=MlDsa
        export CATHEDRAL_FALLBACK_SIGNATURE_ALG=Ed25519
        export CATHEDRAL_DUAL_STACK=true
        ;;
    production-canary)
        export CATHEDRAL_SIGNATURE_ALG=MlDsa
        export CATHEDRAL_FALLBACK_SIGNATURE_ALG=Ed25519
        export CATHEDRAL_DUAL_STACK=true
        export CATHEDRAL_FORCE_PQC=false
        ;;
    production)
        export CATHEDRAL_SIGNATURE_ALG=MlDsa
        export CATHEDRAL_FALLBACK_SIGNATURE_ALG=Ed25519
        export CATHEDRAL_DUAL_STACK=false
        ;;
    *)
        echo "Uso: $0 {staging|production-canary|production}"
        ;;
esac

echo "Deployando com configuração:"
env | grep CATHEDRAL_

# docker-compose up -d --build
