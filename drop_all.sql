/* !!! THIS DROPS ALL TABLES IN THE DATABASE WHICH DELETES ALL DATA IN THE DATABASE !!!
 *
 * ONLY RUN IN DEVELOPMENT!
 */
drop table _sqlx_migrations cascade;
drop table entry cascade;
drop table feed cascade;
drop table users cascade;
drop table user_email_verification_token cascade;
drop type feed_type;
drop collation case_insensitive;
