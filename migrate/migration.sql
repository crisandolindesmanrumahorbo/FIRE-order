CREATE TABLE orders (
  order_id SERIAL PRIMARY KEY,
  product_symbol VARCHAR(10) NOT NULL,
  product_name VARCHAR(100) NOT NULL,
  side VARCHAR(1) NOT NULL,
  price DECIMAL(10,2) NOT NULL,
  lot INT NOT NULL,
  expiry VARCHAR(50) NOT NULL,
  created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
  user_id INT NOT NULL,
  product_id INT NOT NULL,
  FOREIGN KEY (user_id) REFERENCES users(user_id),
  FOREIGN KEY (product_id) REFERENCES products(product_id)
);
