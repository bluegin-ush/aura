#!/bin/bash
# test_api.sh - Tests para MotoStock API

BASE="http://localhost:8081"
PASS=0
FAIL=0

# Colores
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

test_endpoint() {
    local name="$1"
    local expected="$2"
    local result="$3"

    if echo "$result" | grep -q "$expected"; then
        echo -e "${GREEN}✓${NC} $name"
        ((PASS++))
    else
        echo -e "${RED}✗${NC} $name"
        echo "  Expected: $expected"
        echo "  Got: $result"
        ((FAIL++))
    fi
}

echo "=== MotoStock API Tests ==="
echo ""

# Test 1: Health check
R=$(curl -s $BASE/health)
test_endpoint "GET /health" "ok" "$R"

# Test 2: List parts
R=$(curl -s $BASE/parts)
test_endpoint "GET /parts - lista repuestos" "Aceite" "$R"

# Test 3: Get single part
R=$(curl -s $BASE/part/1)
test_endpoint "GET /part/1 - obtener repuesto" "ACE-001" "$R"

# Test 4: Get non-existent part (returns null/empty)
R=$(curl -s $BASE/part/999)
test_endpoint "GET /part/999 - no encontrado" "null" "$R"

# Test 5: Create part (using path params)
R=$(curl -s -X POST "$BASE/part/NEW-001/Repuesto%20Nuevo/Generic/10.50/5/2")
test_endpoint "POST /part - crear repuesto" "created" "$R"

# Test 6: Verify created part exists
R=$(curl -s "$BASE/partsSearch?q=Nuevo")
test_endpoint "GET /partsSearch?q=Nuevo - buscar creado" "NEW-001" "$R"

# Test 7: Update part price (using path params)
R=$(curl -s -X PUT "$BASE/part/1/30.00")
test_endpoint "PUT /part/1/30.00 - actualizar precio" "updated" "$R"

# Test 8: Verify update
R=$(curl -s $BASE/part/1)
test_endpoint "GET /part/1 - verificar actualizacion" "30" "$R"

# Test 9: Low stock (BUJ-001 tiene stock 5 < min_stock 10)
R=$(curl -s $BASE/partsLowStock)
test_endpoint "GET /partsLowStock - stock bajo" "BUJ-001" "$R"

# Test 10: Search by name
R=$(curl -s "$BASE/partsSearch?q=aceite")
test_endpoint "GET /partsSearch?q=aceite - buscar por nombre" "Aceite" "$R"

# Test 11: Search by code
R=$(curl -s "$BASE/partsSearch?q=FIL")
test_endpoint "GET /partsSearch?q=FIL - buscar por codigo" "Filtro" "$R"

# Test 12: Delete part (el que creamos, ID 5)
R=$(curl -s -X DELETE $BASE/part/5)
test_endpoint "DELETE /part/5 - eliminar repuesto" "deleted" "$R"

# Test 13: Verify deletion (returns null)
R=$(curl -s $BASE/part/5)
test_endpoint "GET /part/5 - verificar eliminacion" "null" "$R"

echo ""
echo "================================"
echo -e "Results: ${GREEN}$PASS passed${NC}, ${RED}$FAIL failed${NC}"
echo "================================"

# Exit con codigo de error si hay fallos
if [ $FAIL -gt 0 ]; then
    exit 1
fi
