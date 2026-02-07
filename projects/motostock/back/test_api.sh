#!/bin/bash
# test_api.sh - Tests para MotoStock API

BASE="http://localhost:8081"
PASS=0
FAIL=0

GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

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

# ==================== HEALTH ====================
echo -e "${BLUE}--- Health ---${NC}"
R=$(curl -s $BASE/health)
test_endpoint "GET /health" "ok" "$R"

# ==================== PARTS ====================
echo -e "${BLUE}--- Parts ---${NC}"

R=$(curl -s $BASE/parts)
test_endpoint "GET /parts" "Aceite" "$R"

R=$(curl -s $BASE/part/1)
test_endpoint "GET /part/1" "ACE-001" "$R"

R=$(curl -s $BASE/part/999)
test_endpoint "GET /part/999 - not found" "null" "$R"

R=$(curl -s -X POST "$BASE/part/NEW-001/Repuesto%20Nuevo/Generic/10.50/5/2")
test_endpoint "POST /part - crear" "created" "$R"

R=$(curl -s "$BASE/partsSearch?q=Nuevo")
test_endpoint "GET /partsSearch" "NEW-001" "$R"

R=$(curl -s $BASE/partsLowStock)
test_endpoint "GET /partsLowStock" "BUJ-001" "$R"

# ==================== MOTOS ====================
echo -e "${BLUE}--- Motos ---${NC}"

R=$(curl -s $BASE/motos)
test_endpoint "GET /motos" "AB123CD" "$R"

R=$(curl -s $BASE/moto/1)
test_endpoint "GET /moto/1" "Juan Perez" "$R"

R=$(curl -s -X POST "$BASE/moto/ZZ999AA/Suzuki/GN%20125/2021/Pedro%20Lopez/1155559999")
test_endpoint "POST /moto - crear" "created" "$R"

R=$(curl -s -X PUT "$BASE/moto/3/Pedro%20Lopez%20Jr/1155550000")
test_endpoint "PUT /moto/3 - actualizar" "updated" "$R"

R=$(curl -s $BASE/motoOrders/1)
test_endpoint "GET /motoOrders/1" "\[" "$R"

# ==================== ORDERS ====================
echo -e "${BLUE}--- Orders ---${NC}"

R=$(curl -s -X POST "$BASE/order/1/Service%2010000km")
test_endpoint "POST /order - crear" "created" "$R"

R=$(curl -s $BASE/order/1)
test_endpoint "GET /order/1" "Service 10000km" "$R"

R=$(curl -s $BASE/orders)
test_endpoint "GET /orders" "pending" "$R"

R=$(curl -s -X PUT "$BASE/orderStatus/1/in_progress")
test_endpoint "PUT /orderStatus - cambiar estado" "in_progress" "$R"

# ==================== ORDER ITEMS ====================
echo -e "${BLUE}--- Order Items ---${NC}"

# Guardar stock antes
STOCK_BEFORE=$(curl -s $BASE/part/1 | grep -o '"stock":[0-9]*' | grep -o '[0-9]*')

R=$(curl -s -X POST "$BASE/orderItem/1/1/2")
test_endpoint "POST /orderItem - agregar 2 aceites" "created" "$R"

R=$(curl -s $BASE/orderItems/1)
test_endpoint "GET /orderItems/1" "ACE-001" "$R"

R=$(curl -s $BASE/orderTotal/1)
test_endpoint "GET /orderTotal/1" "total" "$R"

# Verificar descuento de stock
STOCK_AFTER=$(curl -s $BASE/part/1 | grep -o '"stock":[0-9]*' | grep -o '[0-9]*')
if [ "$((STOCK_BEFORE - 2))" -eq "$STOCK_AFTER" ]; then
    echo -e "${GREEN}✓${NC} Stock descontado correctamente ($STOCK_BEFORE -> $STOCK_AFTER)"
    ((PASS++))
else
    echo -e "${RED}✗${NC} Stock no descontado ($STOCK_BEFORE -> $STOCK_AFTER)"
    ((FAIL++))
fi

# ==================== REPORTS ====================
echo -e "${BLUE}--- Reports ---${NC}"

R=$(curl -s $BASE/reportsInventory)
test_endpoint "GET /reportsInventory" "total_value" "$R"

R=$(curl -s $BASE/reportsLowStock)
test_endpoint "GET /reportsLowStock" "to_order" "$R"

R=$(curl -s $BASE/reportsMonthly)
test_endpoint "GET /reportsMonthly" "revenue" "$R"

# ==================== CLEANUP ====================
echo -e "${BLUE}--- Cleanup ---${NC}"

R=$(curl -s -X DELETE $BASE/order/1)
test_endpoint "DELETE /order/1" "deleted" "$R"

R=$(curl -s -X DELETE $BASE/moto/3)
test_endpoint "DELETE /moto/3" "deleted" "$R"

R=$(curl -s -X DELETE $BASE/part/5)
test_endpoint "DELETE /part/5" "deleted" "$R"

echo ""
echo "================================"
echo -e "Results: ${GREEN}$PASS passed${NC}, ${RED}$FAIL failed${NC}"
echo "================================"

if [ $FAIL -gt 0 ]; then
    exit 1
fi
