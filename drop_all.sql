/* !!! THIS DROPS ALL TABLES IN THE DATABASE WHICH DELETES ALL DATA IN THE DATABASE !!!
 *
 * ONLY RUN IN DEVELOPMENT!
 */
drop table _sqlx_migrations cascade;
drop collation case_insensitive;
drop table entry cascade;
drop table feed cascade;
drop table users cascade;
drop type feed_type;
